#!/bin/bash

# Deep Agent HTTP Server API Test Script
echo "🧪 Testing Deep Agent HTTP Server API"
echo "====================================="

BASE_URL="http://localhost:3000/api/v1"

# Test 1: Health Check
echo "1️⃣ Testing health endpoint..."
curl -s "$BASE_URL/health" | jq '.'
echo -e "\n"

# Test 2: Get available agents
echo "2️⃣ Testing agents info endpoint..."
curl -s "$BASE_URL/agents" | jq '.'
echo -e "\n"

# Test 3: Simple chat message
echo "3️⃣ Testing simple chat..."
RESPONSE=$(curl -s -X POST "$BASE_URL/chat" \
  -H 'Content-Type: application/json' \
  -d '{"message": "Hello! Can you help me understand what you can do?"}')

echo "$RESPONSE" | jq '.'
SESSION_ID=$(echo "$RESPONSE" | jq -r '.session_id')
echo "📝 Session ID: $SESSION_ID"
echo -e "\n"

# Test 4: Research question
echo "4️⃣ Testing research capabilities..."
curl -s -X POST "$BASE_URL/chat" \
  -H 'Content-Type: application/json' \
  -d '{
    "message": "What is quantum computing and what are its main applications?",
    "session_id": "'$SESSION_ID'",
    "agent_type": "research"
  }' | jq '.'
echo -e "\n"

# Test 5: Get session info
echo "5️⃣ Testing session info..."
curl -s "$BASE_URL/sessions/$SESSION_ID" | jq '.'
echo -e "\n"

# Test 6: List all sessions
echo "6️⃣ Testing sessions list..."
curl -s "$BASE_URL/sessions" | jq '.'
echo -e "\n"

echo "✅ API tests completed!"
