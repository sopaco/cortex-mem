use crate::{
    config::{Config, QdrantConfig},
    error::Result,
    llm::LLMClient,
    vector_store::{QdrantVectorStore, VectorStore},
};
use tracing::info;

/// Initialize the memory system with auto-detected embedding dimensions
pub async fn initialize_memory_system(config: &Config) -> Result<(Box<dyn VectorStore>, Box<dyn LLMClient>)> {
    // Create LLM client first
    let llm_client = crate::llm::create_llm_client(&config.llm, &config.embedding)?;
    
    // Create vector store with auto-detection if needed
    let vector_store: Box<dyn VectorStore> = if config.qdrant.embedding_dim.is_some() {
        info!("Using configured embedding dimension: {:?}", config.qdrant.embedding_dim);
        Box::new(QdrantVectorStore::new(&config.qdrant).await?)
    } else {
        info!("Auto-detecting embedding dimension...");
        Box::new(QdrantVectorStore::new_with_llm_client(&config.qdrant, llm_client.as_ref()).await?)
    };
    
    Ok((vector_store, llm_client))
}

/// Create a QdrantConfig with auto-detected embedding dimension
pub async fn create_auto_config(
    base_config: &QdrantConfig,
    llm_client: &dyn LLMClient,
) -> Result<QdrantConfig> {
    let mut config = base_config.clone();
    
    if config.embedding_dim.is_none() {
        info!("Auto-detecting embedding dimension for configuration...");
        let test_embedding = llm_client.embed("test").await?;
        let detected_dim = test_embedding.len();
        info!("Detected embedding dimension: {}", detected_dim);
        config.embedding_dim = Some(detected_dim);
    }
    
    Ok(config)
}