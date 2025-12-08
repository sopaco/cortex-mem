pub mod manager;
pub mod extractor;
pub mod updater;
pub mod importance;
pub mod deduplication;
pub mod classification;
pub mod utils;
pub mod prompts;

// Optimization modules
pub mod optimizer;
pub mod optimization_detector;
pub mod optimization_analyzer;
pub mod execution_engine;
pub mod result_reporter;
pub mod optimization_plan;

pub use manager::*;
pub use extractor::*;
pub use updater::*;
pub use importance::*;
pub use deduplication::*;
pub use classification::*;
pub use utils::*;
pub use prompts::*;

pub use optimizer::*;
pub use optimization_detector::*;
pub use optimization_analyzer::*;
pub use execution_engine::*;
pub use result_reporter::*;
pub use optimization_plan::*;
