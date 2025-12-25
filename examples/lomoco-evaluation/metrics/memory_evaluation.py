"""
Memory System Evaluation Metrics

This module provides metrics specifically designed for evaluating memory systems,
including retrieval accuracy, recall, precision, and ranking quality.

Key improvements over traditional NLP metrics:
- Recall@K: Measures if relevant information is retrieved within top K results
- MRR (Mean Reciprocal Rank): Measures how well the system ranks relevant memories
- Precision@K: Measures the proportion of relevant memories in top K results
- Normalized Discounted Cumulative Gain (NDCG): Measures ranking quality
"""

import json
import numpy as np
from collections import defaultdict
from typing import List, Dict, Tuple, Set
from sentence_transformers import SentenceTransformer
import statistics


class MemorySystemEvaluator:
    """Evaluator for memory systems with comprehensive metrics"""
    
    def __init__(self, embedding_model="all-MiniLM-L6-v2"):
        """Initialize the evaluator with an embedding model for semantic similarity"""
        try:
            self.sentence_model = SentenceTransformer(embedding_model)
        except Exception as e:
            print(f"Warning: Could not load SentenceTransformer model: {e}")
            self.sentence_model = None
    
    def calculate_semantic_similarity(self, text1: str, text2: str) -> float:
        """Calculate semantic similarity between two texts using sentence embeddings"""
        if self.sentence_model is None:
            return 0.0
        
        try:
            emb1 = self.sentence_model.encode([text1], convert_to_tensor=True)
            emb2 = self.sentence_model.encode([text2], convert_to_tensor=True)
            from sentence_transformers.util import pytorch_cos_sim
            similarity = pytorch_cos_sim(emb1, emb2).item()
            return float(similarity)
        except Exception as e:
            print(f"Error calculating semantic similarity: {e}")
            return 0.0
    
    def extract_keywords(self, text: str) -> Set[str]:
        """Extract keywords from text for relevance matching"""
        # Simple keyword extraction - can be enhanced with NLP libraries
        import re
        words = re.findall(r'\b\w+\b', text.lower())
        # Filter out common stop words
        stop_words = {'the', 'a', 'an', 'is', 'are', 'was', 'were', 'be', 'been', 'being',
                     'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'should',
                     'could', 'may', 'might', 'must', 'shall', 'can', 'to', 'of', 'in',
                     'for', 'on', 'at', 'from', 'by', 'with', 'as', 'and', 'or', 'but',
                     'not', 'no', 'yes', 'it', 'this', 'that', 'these', 'those', 'i',
                     'you', 'he', 'she', 'we', 'they', 'what', 'which', 'who', 'when',
                     'where', 'why', 'how', 'said', 'says', 'told', 'asked'}
        return {word for word in words if len(word) > 2 and word not in stop_words}
    
    def is_relevant(self, memory: Dict, gold_answer: str, question: str, 
                   similarity_threshold: float = 0.3) -> bool:
        """
        Determine if a retrieved memory is relevant to the question and answer
        
        Args:
            memory: Dictionary containing memory content
            gold_answer: The ground truth answer
            question: The question asked
            similarity_threshold: Minimum semantic similarity score to consider relevant
        
        Returns:
            True if memory is relevant, False otherwise
        """
        memory_content = memory.get('memory', '') or memory.get('content', '')
        
        # Method 1: Keyword overlap
        answer_keywords = self.extract_keywords(gold_answer)
        question_keywords = self.extract_keywords(question)
        memory_keywords = self.extract_keywords(memory_content)
        
        # Check if memory contains keywords from both question and answer
        answer_overlap = len(answer_keywords & memory_keywords)
        question_overlap = len(question_keywords & memory_keywords)
        
        keyword_score = (answer_overlap + question_overlap) / max(len(answer_keywords | question_keywords), 1)
        
        # Method 2: Semantic similarity
        semantic_score = self.calculate_semantic_similarity(memory_content, gold_answer)
        
        # Combined relevance score
        relevance = (keyword_score * 0.6) + (semantic_score * 0.4)
        
        return relevance >= similarity_threshold
    
    def calculate_recall_at_k(self, retrieved_memories: List[Dict], gold_answer: str,
                             question: str, k: int = 5, **kwargs) -> float:
        """
        Calculate Recall@K - whether at least one relevant memory is in top K results
        
        Args:
            retrieved_memories: List of retrieved memories
            gold_answer: Ground truth answer
            question: The question
            k: Number of top results to consider
        
        Returns:
            Recall@K score (0.0 or 1.0)
        """
        if not retrieved_memories:
            return 0.0
        
        top_k_memories = retrieved_memories[:k]
        for i, memory in enumerate(top_k_memories):
            if self.is_relevant(memory, gold_answer, question, **kwargs):
                return 1.0
        return 0.0
    
    def calculate_precision_at_k(self, retrieved_memories: List[Dict], gold_answer: str,
                                 question: str, k: int = 5, **kwargs) -> float:
        """
        Calculate Precision@K - proportion of relevant memories in top K results
        
        Args:
            retrieved_memories: List of retrieved memories
            gold_answer: Ground truth answer
            question: The question
            k: Number of top results to consider
        
        Returns:
            Precision@K score (0.0 to 1.0)
        """
        if not retrieved_memories:
            return 0.0
        
        top_k_memories = retrieved_memories[:k]
        relevant_count = sum(
            1 for memory in top_k_memories 
            if self.is_relevant(memory, gold_answer, question, **kwargs)
        )
        
        return relevant_count / len(top_k_memories)
    
    def calculate_mrr(self, retrieved_memories: List[Dict], gold_answer: str,
                     question: str, **kwargs) -> float:
        """
        Calculate Mean Reciprocal Rank - 1/rank of first relevant memory
        
        Args:
            retrieved_memories: List of retrieved memories
            gold_answer: Ground truth answer
            question: The question
        
        Returns:
            MRR score (0.0 to 1.0)
        """
        if not retrieved_memories:
            return 0.0
        
        for i, memory in enumerate(retrieved_memories, start=1):
            if self.is_relevant(memory, gold_answer, question, **kwargs):
                return 1.0 / i
        
        return 0.0
    
    def calculate_ndcg_at_k(self, retrieved_memories: List[Dict], gold_answer: str,
                           question: str, k: int = 5, **kwargs) -> float:
        """
        Calculate Normalized Discounted Cumulative Gain at K
        
        Args:
            retrieved_memories: List of retrieved memories
            gold_answer: Ground truth answer
            question: The question
            k: Number of top results to consider
        
        Returns:
            NDCG@K score (0.0 to 1.0)
        """
        if not retrieved_memories:
            return 0.0
        
        # Calculate DCG
        dcg = 0.0
        for i, memory in enumerate(retrieved_memories[:k], start=1):
            if self.is_relevant(memory, gold_answer, question, **kwargs):
                # Binary relevance: 1 if relevant, 0 otherwise
                relevance = 1
                dcg += relevance / np.log2(i + 1)
        
        # Calculate ideal DCG (all top K results are relevant)
        ideal_dcg = sum(1.0 / np.log2(i + 2) for i in range(min(k, len(retrieved_memories))))
        
        if ideal_dcg == 0:
            return 0.0
        
        return dcg / ideal_dcg
    
    def calculate_answer_quality(self, generated_answer: str, gold_answer: str,
                                 retrieved_memories: List[Dict], **kwargs) -> Dict[str, float]:
        """
        Calculate answer quality metrics
        
        Args:
            generated_answer: The answer generated by the system
            gold_answer: The ground truth answer
            retrieved_memories: Memories used to generate the answer
        
        Returns:
            Dictionary of answer quality scores
        """
        # Semantic similarity with gold answer
        semantic_similarity = self.calculate_semantic_similarity(generated_answer, gold_answer)
        
        # Exact match (case-insensitive)
        exact_match = 1.0 if generated_answer.strip().lower() == gold_answer.strip().lower() else 0.0
        
        # Keyword overlap F1 score
        generated_keywords = self.extract_keywords(generated_answer)
        gold_keywords = self.extract_keywords(gold_answer)
        
        if not generated_keywords or not gold_keywords:
            f1_score = 0.0
        else:
            common_keywords = generated_keywords & gold_keywords
            precision = len(common_keywords) / len(generated_keywords)
            recall = len(common_keywords) / len(gold_keywords)
            f1_score = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0.0
        
        # Check if answer is based on retrieved memories
        memory_based = 0.0
        if retrieved_memories:
            memory_texts = " ".join([m.get('memory', '') or m.get('content', '') for m in retrieved_memories])
            if self.calculate_semantic_similarity(generated_answer, memory_texts) > 0.3:
                memory_based = 1.0
        
        return {
            "semantic_similarity": semantic_similarity,
            "exact_match": exact_match,
            "keyword_f1": f1_score,
            "memory_based": memory_based
        }
    
    def evaluate_conversation(self, result_data: Dict, gold_data: Dict) -> Dict[str, float]:
        """
        Evaluate a single conversation with comprehensive metrics
        
        Args:
            result_data: Result data from the memory system
            gold_data: Ground truth data from the dataset
        
        Returns:
            Dictionary of evaluation scores
        """
        metrics = {
            "recall_at_1": 0.0,
            "recall_at_3": 0.0,
            "recall_at_5": 0.0,
            "recall_at_10": 0.0,
            "precision_at_1": 0.0,
            "precision_at_3": 0.0,
            "precision_at_5": 0.0,
            "mrr": 0.0,
            "ndcg_at_5": 0.0,
            "ndcg_at_10": 0.0,
            "answer_semantic_similarity": 0.0,
            "answer_exact_match": 0.0,
            "answer_keyword_f1": 0.0,
        }
        
        question = result_data.get("question", "")
        gold_answer = result_data.get("answer", "")
        generated_answer = result_data.get("response", "")
        
        # Combine memories from both speakers
        speaker_1_memories = result_data.get("speaker_1_memories", [])
        speaker_2_memories = result_data.get("speaker_2_memories", [])
        all_memories = speaker_1_memories + speaker_2_memories
        
        if not all_memories:
            # Calculate answer quality metrics even if no memories retrieved
            answer_quality = self.calculate_answer_quality(generated_answer, gold_answer, [])
            metrics.update(answer_quality)
            return metrics
        
        # Calculate retrieval metrics
        metrics["recall_at_1"] = self.calculate_recall_at_k(all_memories, gold_answer, question, k=1)
        metrics["recall_at_3"] = self.calculate_recall_at_k(all_memories, gold_answer, question, k=3)
        metrics["recall_at_5"] = self.calculate_recall_at_k(all_memories, gold_answer, question, k=5)
        metrics["recall_at_10"] = self.calculate_recall_at_k(all_memories, gold_answer, question, k=10)
        
        metrics["precision_at_1"] = self.calculate_precision_at_k(all_memories, gold_answer, question, k=1)
        metrics["precision_at_3"] = self.calculate_precision_at_k(all_memories, gold_answer, question, k=3)
        metrics["precision_at_5"] = self.calculate_precision_at_k(all_memories, gold_answer, question, k=5)
        
        metrics["mrr"] = self.calculate_mrr(all_memories, gold_answer, question)
        
        metrics["ndcg_at_5"] = self.calculate_ndcg_at_k(all_memories, gold_answer, question, k=5)
        metrics["ndcg_at_10"] = self.calculate_ndcg_at_k(all_memories, gold_answer, question, k=10)
        
        # Calculate answer quality metrics
        answer_quality = self.calculate_answer_quality(generated_answer, gold_answer, all_memories)
        metrics["answer_semantic_similarity"] = answer_quality["semantic_similarity"]
        metrics["answer_exact_match"] = answer_quality["exact_match"]
        metrics["answer_keyword_f1"] = answer_quality["keyword_f1"]
        
        return metrics
    
    def evaluate_dataset(self, results_file: str, dataset_file: str) -> Dict:
        """
        Evaluate entire dataset with comprehensive metrics and statistics
        
        Args:
            results_file: Path to results file from memory system
            dataset_file: Path to ground truth dataset file
        
        Returns:
            Dictionary containing aggregated metrics and statistics
        """
        # Load data
        with open(results_file, 'r') as f:
            results_data = json.load(f)
        
        with open(dataset_file, 'r') as f:
            dataset_data = json.load(f)
        
        # Store all metrics
        all_metrics = defaultdict(list)
        category_metrics = defaultdict(lambda: defaultdict(list))
        
        # Process each question
        for conv_id, results_list in results_data.items():
            conv_idx = int(conv_id)
            if conv_idx >= len(dataset_data):
                continue
            
            conv_data = dataset_data[conv_idx]
            
            for i, result_item in enumerate(results_list):
                # Evaluate
                metrics = self.evaluate_conversation(result_item, conv_data)
                
                # Store metrics
                category = result_item.get("category", 0)
                for metric_name, value in metrics.items():
                    all_metrics[metric_name].append(value)
                    category_metrics[category][metric_name].append(value)
        
        # Calculate statistics
        aggregated_results = {}
        
        # Overall statistics
        overall_stats = {}
        for metric_name, values in all_metrics.items():
            if values:
                overall_stats[metric_name] = {
                    "mean": np.mean(values),
                    "std": np.std(values) if len(values) > 1 else 0.0,
                    "median": np.median(values),
                    "min": np.min(values),
                    "max": np.max(values),
                    "count": len(values),
                    "confidence_interval_95": self._calculate_confidence_interval(values, 0.95)
                }
        
        aggregated_results["overall"] = overall_stats
        
        # Per-category statistics
        for category, cat_metrics in category_metrics.items():
            cat_stats = {}
            for metric_name, values in cat_metrics.items():
                if values:
                    cat_stats[metric_name] = {
                        "mean": np.mean(values),
                        "std": np.std(values) if len(values) > 1 else 0.0,
                        "median": np.median(values),
                        "count": len(values),
                    }
            aggregated_results[f"category_{category}"] = cat_stats
        
        return aggregated_results
    
    def _calculate_confidence_interval(self, values: List[float], confidence: float = 0.95) -> Tuple[float, float]:
        """Calculate confidence interval for a list of values"""
        if len(values) < 2:
            return (0.0, 0.0)
        
        mean = np.mean(values)
        std_error = statistics.stdev(values) / np.sqrt(len(values))
        
        from scipy import stats
        t_value = stats.t.ppf((1 + confidence) / 2, len(values) - 1)
        
        margin_of_error = t_value * std_error
        
        return (mean - margin_of_error, mean + margin_of_error)


def run_evaluation(results_file: str, dataset_file: str, output_file: str = "evaluation_metrics_v2.json"):
    """Run comprehensive evaluation"""
    evaluator = MemorySystemEvaluator()
    
    print(f"Evaluating {results_file} against {dataset_file}...")
    results = evaluator.evaluate_dataset(results_file, dataset_file)
    
    # Save results
    with open(output_file, 'w') as f:
        json.dump(results, f, indent=2)
    
    print(f"\n✅ Evaluation complete! Results saved to {output_file}")
    
    # Print summary
    print("\n📊 Overall Results:")
    print("-" * 60)
    for metric, stats in results.get("overall", {}).items():
        print(f"{metric:30s}: {stats['mean']:.4f} ± {stats['std']:.4f} (95% CI: {stats['confidence_interval_95'][0]:.4f} - {stats['confidence_interval_95'][1]:.4f})")
    
    return results


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Evaluate memory system with comprehensive metrics")
    parser.add_argument("--results", required=True, help="Path to results file")
    parser.add_argument("--dataset", required=True, help="Path to dataset file")
    parser.add_argument("--output", default="evaluation_metrics_v2.json", help="Output file path")
    
    args = parser.parse_args()
    run_evaluation(args.results, args.dataset, args.output)
