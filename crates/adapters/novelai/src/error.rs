use atelier_director::{
    ClientApiErrorContext as DirectorApiErrorContext,
    ClientApiErrorReason as DirectorApiErrorReason, ClientDecodeContext as DirectorDecodeContext,
    ClientDecodeTarget as DirectorDecodeTarget,
    ClientInvalidRequestContext as DirectorInvalidRequestContext,
    ClientInvalidRequestKind as DirectorInvalidRequestKind,
    ClientMetadataContext as DirectorMetadataContext, ClientMetadataKind as DirectorMetadataKind,
    ClientTransportContext as DirectorTransportContext,
    ClientTransportOperation as DirectorTransportOperation, DirectorClientError,
};
use atelier_generation::{
    ClientApiErrorContext as GenerationApiErrorContext,
    ClientApiErrorReason as GenerationApiErrorReason,
    ClientDecodeContext as GenerationDecodeContext, ClientDecodeTarget as GenerationDecodeTarget,
    ClientInvalidRequestContext as GenerationInvalidRequestContext,
    ClientInvalidRequestKind as GenerationInvalidRequestKind,
    ClientMetadataContext as GenerationMetadataContext,
    ClientMetadataKind as GenerationMetadataKind,
    ClientTransportContext as GenerationTransportContext,
    ClientTransportOperation as GenerationTransportOperation, GenerationClientError,
};
use atelier_secrets::{
    ClientApiErrorContext as SubscriptionApiErrorContext,
    ClientApiErrorReason as SubscriptionApiErrorReason,
    ClientDecodeContext as SubscriptionDecodeContext,
    ClientDecodeTarget as SubscriptionDecodeTarget,
    ClientInvalidRequestContext as SubscriptionInvalidRequestContext,
    ClientInvalidRequestKind as SubscriptionInvalidRequestKind,
    ClientMetadataContext as SubscriptionMetadataContext,
    ClientMetadataKind as SubscriptionMetadataKind,
    ClientTransportContext as SubscriptionTransportContext,
    ClientTransportOperation as SubscriptionTransportOperation, SubscriptionClientError,
};
use atelier_vibe::{
    ClientApiErrorContext as VibeApiErrorContext, ClientApiErrorReason as VibeApiErrorReason,
    ClientDecodeContext as VibeDecodeContext, ClientDecodeTarget as VibeDecodeTarget,
    ClientInvalidRequestContext as VibeInvalidRequestContext,
    ClientInvalidRequestKind as VibeInvalidRequestKind,
    ClientMetadataContext as VibeMetadataContext, ClientMetadataKind as VibeMetadataKind,
    ClientTransportContext as VibeTransportContext,
    ClientTransportOperation as VibeTransportOperation, VibeClientError,
};
mod bridge;
mod client;
mod types;

pub use bridge::map_bridge_error;
pub use client::{
    map_director_error, map_generation_error, map_subscription_error, map_vibe_error,
};
pub use types::*;
