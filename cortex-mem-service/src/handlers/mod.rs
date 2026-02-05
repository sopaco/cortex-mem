pub mod health;
pub mod filesystem;
pub mod sessions;
pub mod search;
pub mod automation;

#[cfg(all(test, feature = "vector-search"))]
mod search_integration_tests;
