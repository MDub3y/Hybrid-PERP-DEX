Hybrid-PERP-DEX
===============

The Hybrid Perpetual Decentralized Exchange (DEX) is a high-throughput trading platform built to handle high-frequency derivatives trading. It utilizes a **Hybrid Centralized-Matching / Decentralized-Settlement** architecture, ensuring that while orders are matched with millisecond latency off-chain, user funds and trade finality remain non-custodial and verified on-chain.

Core Architecture
-----------------

The system is split into two primary layers: the **Off-Chain Engine** (performance layer) and the **On-Chain Settlement** (security layer).

### 1\. Off-Chain Matching Engine (Rust)

The high-performance core responsible for maintaining the order book and matching buyers and sellers without the latency of block times.

*   **API Router:** Handles incoming REST/WebSocket requests for order placement, cancellations, and market data.
    
*   **Matching Engine:** A deterministic Rust-based engine that matches limit and market orders.
    
*   **Message Bus (Redis):** Orchestrates communication between services using ORDER\_QUEUE and SETTLEMENT\_QUEUE.
    
*   **Persistence:** Trade history and user state are indexed in **PostgreSQL** for fast retrieval.
    

### 2\. On-Chain Settlement (Solana/Anchor)

The source of truth for all financial state. No funds can move without a valid cryptographic proof from the matching engine.

*   **Margin Accounts:** Program Derived Addresses (PDAs) that manage user collateral and open positions.
    
*   **Signature Introspection:** Uses Ed25519 verification to ensure that every settlement instruction was signed by the authorized off-chain matching engine.
    
*   **Risk Engine:** Validates PnL (Profit and Loss) and margin requirements before updating on-chain state.
    
*   **Liquidations:** A permissionless logic flow that allows the protocol to maintain solvency by closing under-collateralized positions.
    

Technical Workflow
------------------

1.  **Order Placement:** A user signs a transaction and sends it to the API Router.
    
2.  **Matching:** The Matching Engine processes the order. If a match occurs, a "Trade Event" is generated.
    
3.  **Sequencing:** The **Settlement Worker** (the bridge) picks up the trade from the Redis queue.
    
4.  **Verification:** The worker submits the match to the Solana program.
    
5.  **Settlement:** The Solana program verifies the engine's signature, checks the user's margin, and updates the on-chain position/balance.
    

Features
--------

*   **Low Latency:** Millisecond-level order execution via Rust-based off-chain matching.
    
*   **Non-Custodial:** Users retain control of their funds; the matching engine cannot withdraw collateral without a user-signed trade.
    
*   **Scalability:** Offloads heavy computation (order book sorting, matching) from the blockchain.
    
*   **Security:** Built-in protection against unauthorized state updates via cryptographic introspection.
    

**Reverse engineering, and understanding by building a prototype.**