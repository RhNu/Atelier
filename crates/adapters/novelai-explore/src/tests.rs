use super::*;
use atelier_explore::novelai::{NovelAiExplorePeriod, NovelAiExploreSort};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

const POST_ID: &str = "00000000-0000-0000-0000-000000000001";

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
fn query() -> NovelAiExploreQuery {
    NovelAiExploreQuery {
        tags: vec!["blue sky".into()],
        sort: NovelAiExploreSort::New,
        period: None,
        creator_id: None,
        random_salt: None,
    }
}
fn post() -> Value {
    json!({"id":POST_ID,"type":1,"moderation_status":1,"deleted":false,"title":"Sky","created_at":"2026-08-31T00:00:00Z","image":{"width":832,"height":1216,"nai_metadata":"{bad"}})
}

fn serve(responses: Vec<(u16, String, String)>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, headers, body) in responses {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let count = socket.read(&mut chunk).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&chunk[..count]);
                let request = String::from_utf8_lossy(&bytes);
                if let Some(end) = request.find("\r\n\r\n") {
                    let length = request[..end]
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8(bytes).unwrap());
            write!(socket,"HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",body.len()).unwrap();
        }
        requests
    });
    (url, task)
}

#[test]
fn public_search_is_anonymous_and_metadata_failure_is_local() {
    let body = json!({"results":[post()],"pagination":{"limit":PAGE_SIZE,"offset":0,"total":2}})
        .to_string();
    let (url, server) = serve(vec![(
        200,
        "Content-Type: application/json\r\n".into(),
        body,
    )]);
    let page = runtime()
        .block_on(
            NovelAiExploreClient::build(&url)
                .unwrap()
                .search(query(), None),
        )
        .unwrap();
    assert_eq!(page.next_cursor, Some(ExploreCursor::Offset(1)));
    assert_eq!(
        page.items[0].metadata.status,
        atelier_explore::novelai::ExploreMetadataStatus::Invalid
    );
    assert_eq!(page.items[0].like_count, None);
    let request = &server.join().unwrap()[0];
    assert!(request.starts_with("POST /post/search"));
    assert!(!request.to_lowercase().contains("authorization:"));
    assert!(!request.to_lowercase().contains("x-api-key:"));
    let payload: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(payload["selectors"][0]["value"], "blue sky");
    assert_eq!(payload["selectors"][1]["value"], "1");
}

#[test]
fn hidden_rows_advance_pagination_without_becoming_visible() {
    let mut hidden = post();
    hidden["deleted"] = json!(true);
    let body = json!({"results":[hidden],"pagination":{"limit":PAGE_SIZE,"offset":40,"total":100}})
        .to_string();
    let (url, server) = serve(vec![(200, String::new(), body)]);
    let page = runtime()
        .block_on(
            NovelAiExploreClient::build(&url)
                .unwrap()
                .search(query(), Some(ExploreCursor::Offset(40))),
        )
        .unwrap();
    assert!(page.items.is_empty());
    assert_eq!(page.next_cursor, Some(ExploreCursor::Offset(41)));
    server.join().unwrap();
}

#[test]
fn random_salt_and_period_survive_offset_changes() {
    let random = NovelAiExploreQuery {
        tags: vec![],
        sort: NovelAiExploreSort::Random,
        period: Some(NovelAiExplorePeriod::Week),
        creator_id: None,
        random_salt: Some("abc123".into()),
    };
    let first = search_body(&random, 0);
    let second = search_body(&random, 40);
    assert_eq!(first["selectors"], second["selectors"]);
    assert_eq!(
        first["selectors"],
        json!([{"field":"random","value":"week:abc123"}])
    );
    assert_eq!(second["pagination"]["offset"], 40);
}

#[test]
fn rate_limits_keep_retry_after_without_automatic_retries() {
    let (url, server) = serve(vec![(429, "Retry-After: 30\r\n".into(), String::new())]);
    let client = NovelAiExploreClient::build(&url).unwrap();
    let error = runtime()
        .block_on(client.search(query(), None))
        .unwrap_err();
    assert_eq!(error.kind, ExploreErrorKind::RateLimited);
    assert_eq!(error.retry_after_seconds, Some(30));
    assert_eq!(server.join().unwrap().len(), 1);
    assert_eq!(
        runtime()
            .block_on(client.search(query(), None))
            .unwrap_err()
            .kind,
        ExploreErrorKind::RateLimited
    );
}

#[test]
fn redirects_and_unapproved_details_are_rejected() {
    let mut hidden = post();
    hidden["moderation_status"] = json!(0);
    let (url, server) = serve(vec![
        (200, String::new(), hidden.to_string()),
        (
            302,
            "Location: https://example.com/private\r\n".into(),
            String::new(),
        ),
    ]);
    let client = NovelAiExploreClient::build(&url).unwrap();
    let rt = runtime();
    assert_eq!(
        rt.block_on(client.detail(POST_ID)).unwrap_err().kind,
        ExploreErrorKind::NotFound
    );
    assert_eq!(
        rt.block_on(client.detail(POST_ID)).unwrap_err().kind,
        ExploreErrorKind::InvalidResponse
    );
    server.join().unwrap();
}

#[test]
fn media_rejects_non_image_responses() {
    let (url, server) = serve(vec![
        (200, String::new(), post().to_string()),
        (
            200,
            "Content-Type: text/html\r\n".into(),
            "not an image".into(),
        ),
    ]);
    let client = NovelAiExploreClient::build(&url).unwrap();
    assert_eq!(
        runtime()
            .block_on(client.media(POST_ID, ExploreMediaVariant::Thumbnail))
            .unwrap_err()
            .kind,
        ExploreErrorKind::MediaRejected
    );
    server.join().unwrap();
}

#[test]
fn oversized_responses_and_bad_ids_are_rejected() {
    let (url, server) = serve(vec![(200, String::new(), "x".repeat(MAX_JSON_BYTES + 1))]);
    let client = NovelAiExploreClient::build(&url).unwrap();
    let rt = runtime();
    assert_eq!(
        rt.block_on(client.detail("../secret")).unwrap_err().kind,
        ExploreErrorKind::InvalidRequest
    );
    assert_eq!(
        rt.block_on(client.search(query(), None)).unwrap_err().kind,
        ExploreErrorKind::MediaRejected
    );
    server.join().unwrap();
}

#[test]
fn list_approval_can_serve_media_but_does_not_replace_full_detail() {
    let page = json!({"results":[post()],"pagination":{"limit":PAGE_SIZE,"offset":0,"total":1}});
    let mut detail = post();
    detail["like_count"] = json!(42);
    let (url, server) = serve(vec![
        (200, String::new(), page.to_string()),
        (
            200,
            "Content-Type: image/png\r\n".into(),
            "invalid png".into(),
        ),
        (200, String::new(), detail.to_string()),
    ]);
    let client = NovelAiExploreClient::build(&url).unwrap();
    let rt = runtime();
    assert_eq!(
        rt.block_on(client.search(query(), None)).unwrap().items[0].like_count,
        None
    );
    assert_eq!(
        rt.block_on(client.media(POST_ID, ExploreMediaVariant::Thumbnail))
            .unwrap_err()
            .kind,
        ExploreErrorKind::MediaRejected
    );
    assert_eq!(
        rt.block_on(client.detail(POST_ID)).unwrap().like_count,
        Some(42)
    );
    let requests = server.join().unwrap();
    assert!(requests[1].starts_with("GET /post/thumbnail/"));
}

#[test]
#[ignore = "Manual low-frequency check of the undocumented public read-only service"]
fn live_public_read_only_smoke() {
    runtime().block_on(async {
        let client = NovelAiExploreClient::new().unwrap();
        let mut request = query();
        request.tags.clear();
        let page = client.search(request.clone(), None).await.unwrap();
        let first = page
            .items
            .first()
            .expect("public gallery returned no visible posts");
        let detail = client.detail(&first.id).await.unwrap();
        assert_eq!(first.id, detail.id);
        let thumbnail = client
            .media(&first.id, ExploreMediaVariant::Thumbnail)
            .await
            .unwrap();
        assert!(!thumbnail.bytes.is_empty());
        for sort in [
            NovelAiExploreSort::Top,
            NovelAiExploreSort::Hot,
            NovelAiExploreSort::Random,
        ] {
            request.sort = sort;
            request.period = Some(NovelAiExplorePeriod::Week);
            request.random_salt = (sort == NovelAiExploreSort::Random).then(|| "abc123".into());
            let page = client.search(request.clone(), None).await.unwrap();
            assert!(!page.items.is_empty());
            if sort == NovelAiExploreSort::Random {
                client
                    .search(request.clone(), page.next_cursor)
                    .await
                    .unwrap();
            }
        }
    });
}
