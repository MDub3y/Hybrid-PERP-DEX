#!/bin/bash
# Simulates high-frequency order placement
for i in {1..100}
do
   SIDE=$(shuf -e BUY SELL -n 1)
   PRICE=$(awk 'BEGIN{srand(); printf "%.2f", 95+rand()*10}')
   curl -X POST http://localhost:7000/order \
     -H "Content-Type: application/json" \
     -d "{\"user_id\": \"user_$i\", \"price\": \"$PRICE\", \"quantity\": \"1.0\", \"side\": \"$SIDE\"}"
done