use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use super::*;

#[test]
fn raw_post_maps_all_tag_groups() {
    let post = DanbooruPost::try_from(RawPost {
        id: 7,
        created_at: "2026-01-01T00:00:00Z".to_owned(),
        rating: "s".to_owned(),
        image_width: 1024,
        image_height: 768,
        score: 4,
        fav_count: 3,
        file_ext: "jpg".to_owned(),
        file_size: 42,
        source: String::new(),
        preview_file_url: Some("https://cdn.donmai.us/preview.jpg".to_owned()),
        large_file_url: Some("https://cdn.donmai.us/sample.jpg".to_owned()),
        tag_string_artist: "artist_a".to_owned(),
        tag_string_copyright: "series_a".to_owned(),
        tag_string_character: "character_a".to_owned(),
        tag_string_general: "1girl blue_eyes".to_owned(),
        tag_string_meta: "highres".to_owned(),
    })
    .unwrap();
    assert_eq!(post.rating, DanbooruRating::Sensitive);
    assert_eq!(post.general_tags, ["1girl", "blue_eyes"]);
    assert!(post.source_url.is_none());
}

#[test]
fn media_validation_rejects_non_cdn_hosts() {
    let base = Url::parse(DEFAULT_BASE_URL).unwrap();
    let url = Url::parse("https://example.com/image.jpg").unwrap();
    assert_eq!(
        validate_media_url(&url, &base).unwrap_err().kind,
        DanbooruErrorKind::MediaRejected
    );
}

#[test]
fn credentials_debug_output_redacts_key() {
    let credentials = DanbooruCredentials {
        username: "alice".to_owned(),
        api_key: atelier_secrets::SecretValue::new("secret"),
    };
    let output = format!("{credentials:?}");
    assert!(!output.contains("secret"));
}

#[test]
fn profile_uses_basic_auth_and_identifying_user_agent() {
    let (base_url, server) = serve(vec![json_response(
        200,
        r#"{"name":"alice","level_string":"Member"}"#,
        &[],
    )]);
    let client = ReqwestDanbooruClient::with_base_url(&base_url).unwrap();
    let credentials = DanbooruCredentials {
        username: "alice".to_owned(),
        api_key: atelier_secrets::SecretValue::new("secret"),
    };

    let profile = runtime().block_on(client.profile(&credentials)).unwrap();
    let requests = server.join().unwrap();

    assert_eq!(profile.level.as_deref(), Some("Member"));
    assert!(requests[0].contains("authorization: basic ywxpy2u6c2vjcmv0"));
    assert!(requests[0].contains("user-agent: atelier/"));
    assert!(requests[0].contains("by alice on danbooru"));
    assert!(!requests[0].contains("api_key"));
}

#[test]
fn search_uses_fixed_page_size_cursor_and_cache() {
    let (base_url, server) = serve(vec![json_response(200, "[]", &[])]);
    let client = ReqwestDanbooruClient::with_base_url(&base_url).unwrap();
    let request = DanbooruSearchRequest {
        query: "blue eyes".to_owned(),
        ratings: vec![DanbooruRating::General, DanbooruRating::Sensitive],
        before_id: Some(99),
    };

    let first = runtime()
        .block_on(client.search(request.clone(), None))
        .unwrap();
    let second = runtime().block_on(client.search(request, None)).unwrap();
    let requests = server.join().unwrap();

    assert!(first.posts.is_empty());
    assert_eq!(first, second);
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("limit=40"));
    assert!(requests[0].contains("page=b99"));
    assert!(requests[0].contains("rating%3ag%2cs"));
}

#[test]
fn rate_limit_retries_once_and_preserves_retry_details() {
    let (base_url, server) = serve(vec![
        json_response(429, "{}", &[("Retry-After", "0")]),
        json_response(429, "{}", &[("Retry-After", "7")]),
    ]);
    let client = ReqwestDanbooruClient::with_base_url(&base_url).unwrap();
    let credentials = DanbooruCredentials {
        username: "alice".to_owned(),
        api_key: atelier_secrets::SecretValue::new("secret"),
    };

    let error = runtime()
        .block_on(client.profile(&credentials))
        .unwrap_err();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 2);
    assert_eq!(error.kind, DanbooruErrorKind::RateLimited);
    assert_eq!(error.retry_after_seconds, Some(7));
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn serve(responses: Vec<String>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        responses
            .into_iter()
            .map(|response| {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                stream.write_all(response.as_bytes()).unwrap();
                String::from_utf8(request).unwrap().to_ascii_lowercase()
            })
            .collect()
    });
    (format!("http://{address}/"), server)
}

fn json_response(status: u16, body: &str, headers: &[(&str, &str)]) -> String {
    let reason = if status == 200 {
        "OK"
    } else {
        "Too Many Requests"
    };
    let mut extra = String::new();
    for (name, value) in headers {
        write!(&mut extra, "{name}: {value}\r\n").unwrap();
    }
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",
        body.len()
    )
}
