#!/bin/bash
# Test script for cortex-mem-service vector search feature

set -e

BASE_URL="http://localhost:8080"
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "🧪 Testing Cortex-Mem Service Vector Search"
echo "==========================================="
echo ""

# Test 1: Health check
echo -e "${YELLOW}Test 1: Health Check${NC}"
curl -s -X GET "$BASE_URL/health" | jq '.'
echo ""

# Test 2: Filesystem search (always available)
echo -e "${YELLOW}Test 2: Filesystem Search${NC}"
curl -s -X POST "$BASE_URL/api/v2/search" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "test",
    "mode": "filesystem",
    "limit": 5
  }' | jq '.'
echo ""

# Test 3: Vector search (if compiled with feature)
echo -e "${YELLOW}Test 3: Vector Search (may fallback to filesystem)${NC}"
curl -s -X POST "$BASE_URL/api/v2/search" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "test",
    "mode": "vector",
    "limit": 5
  }' | jq '.'
echo ""

# Test 4: Hybrid search (if compiled with feature)
echo -e "${YELLOW}Test 4: Hybrid Search (may fallback to filesystem)${NC}"
curl -s -X POST "$BASE_URL/api/v2/search" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "test",
    "mode": "hybrid",
    "limit": 5
  }' | jq '.'
echo ""

echo -e "${GREEN}✅ All tests completed!${NC}"
echo ""
echo "Note: Vector and Hybrid searches will fallback to filesystem if:"
echo "  - Service not compiled with --features vector-search"
echo "  - Qdrant not configured or not running"
