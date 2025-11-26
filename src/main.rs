use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

// ==========================================
// DESTINATION STRUCTS
// ==========================================
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedBlock {
    pub block_height: u64,
    pub block_time: i64,
    pub blockhash: String,
    pub parent_slot: u64,
    pub previous_blockhash: String,
    pub rewards: Vec<BlockReward>,
    pub transactions: Vec<ParsedTransaction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockReward {
    pub pubkey: String,
    pub lamports: i64,
    pub post_balance: u64,
    pub reward_type: String,
    pub commission: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedTransaction {
    pub signature: String,
    pub fee_payer: String,
    pub is_success: bool,
    pub account_keys: Vec<String>,
    pub instructions: Vec<ParsedInstruction>,
    pub log_messages: Vec<String>,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub fee: u64,
    pub compute_units_consumed: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedInstruction {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: String,
}

// ==========================================
// RAW BLOCK STRUCTS (RPC Input Format)
// ==========================================
#[derive(Debug, Deserialize)]
pub struct RpcBlockResponse {
    pub result: RpcBlockResult,
}

#[derive(Debug, Deserialize)]
pub struct RpcBlockResult {
    #[serde(rename = "blockHeight")]
    pub block_height: u64,
    #[serde(rename = "blockTime")]
    pub block_time: i64,
    pub blockhash: String,
    #[serde(rename = "parentSlot")]
    pub parent_slot: u64,
    #[serde(rename = "previousBlockhash")]
    pub previous_blockhash: String,
    pub rewards: Vec<RpcReward>,
    pub transactions: Vec<RpcBlockTransaction>,
}

#[derive(Debug, Deserialize)]
pub struct RpcReward {
    pub pubkey: String,
    pub lamports: i64,
    #[serde(rename = "postBalance")]
    pub post_balance: u64,
    #[serde(rename = "rewardType")]
    pub reward_type: String,
    pub commission: Option<u8>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcBlockTransaction {
    pub meta: RpcMeta,
    pub transaction: RpcTransactionContainer,
}

// ==========================================
// RAW TRANSACTION STRUCTS (RPC Input)
// ==========================================

#[derive(Debug, Deserialize)]
pub struct RpcResponse {
    pub result: RpcResult,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcResult {
    pub meta: RpcMeta,
    pub transaction: RpcTransactionContainer,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcMeta {
    pub err: Option<serde_json::Value>,
    #[serde(rename = "logMessages")]
    pub log_messages: Vec<String>,
    #[serde(rename = "preBalances")]
    pub pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    pub post_balances: Vec<u64>,
    #[serde(rename = "loadedAddresses")]
    pub loaded_addresses: Option<RpcLoadedAddresses>,
    pub fee: u64,
    #[serde(rename = "computeUnitsConsumed")]
    pub compute_units_consumed: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcLoadedAddresses {
    pub writable: Vec<String>,
    pub readonly: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcTransactionContainer {
    pub signatures: Vec<String>,
    pub message: RpcMessage,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcMessage {
    #[serde(rename = "accountKeys")]
    pub account_keys: Vec<String>,
    pub instructions: Vec<RpcInstruction>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcInstruction {
    #[serde(rename = "programIdIndex")]
    pub program_id_index: usize,
    pub accounts: Vec<usize>,
    pub data: String,
}

// ==========================================
// MAIN LOGIC
// ==========================================

fn main() {
    // println!("=== Testing Single Transaction ===\n");
    // test_transaction();
    
    println!("\n\n=== Testing Block ===\n");
    test_block();
}

// ==========================================
// SINGLE TRANSACTION PARSER
// ==========================================

pub fn test_transaction() {
    let path = "src/json/genesis.json";
    println!("Loading raw RPC JSON from: {}", path);

    let raw_data: RpcResponse = match load_from_json(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to parse Raw JSON: {}", e);
            return;
        }
    };

    println!("-> Raw JSON loaded successfully.");

    let clean_tx = parse_single_transaction(
        raw_data.result.transaction,
        raw_data.result.meta,
    );

    print_transaction_summary(&clean_tx);
}

// ==========================================
// BLOCK PARSER
// ==========================================

pub fn test_block() {
    let path = "src/json/block.json";
    println!("Loading raw Block JSON from: {}", path);

    let raw_block: RpcBlockResponse = match load_from_json(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to parse Block JSON: {}", e);
            return;
        }
    };

    println!("-> Block JSON loaded successfully.");

    let block = raw_block.result;

    // Parse rewards
    let rewards: Vec<BlockReward> = block.rewards.iter().map(|r| {
        BlockReward {
            pubkey: r.pubkey.clone(),
            lamports: r.lamports,
            post_balance: r.post_balance,
            reward_type: r.reward_type.clone(),
            commission: r.commission,
        }
    }).collect();

    // Parse all transactions in the block
    let parsed_txs: Vec<ParsedTransaction> = block.transactions.iter().map(|tx| {
        parse_single_transaction(tx.transaction.clone(), tx.meta.clone())
    }).collect();

    let parsed_block = ParsedBlock {
        block_height: block.block_height,
        block_time: block.block_time,
        blockhash: block.blockhash,
        parent_slot: block.parent_slot,
        previous_blockhash: block.previous_blockhash,
        rewards,
        transactions: parsed_txs,
    };

    print_block_summary(&parsed_block);
}

// ==========================================
// SHARED TRANSACTION PARSING LOGIC
// ==========================================
fn parse_single_transaction(
    tx: RpcTransactionContainer,
    meta: RpcMeta,
) -> ParsedTransaction {
    let message = tx.message;

    // Build the full account list (static + loaded addresses)
    let mut all_account_keys = message.account_keys.clone();
    if let Some(loaded) = &meta.loaded_addresses {
        all_account_keys.extend(loaded.writable.clone());
        all_account_keys.extend(loaded.readonly.clone());
    }

    // Parse instructions
    let parsed_instructions: Vec<ParsedInstruction> = message.instructions.iter().map(|ix| {
        // Resolve Program ID
        let program_id = if ix.program_id_index < all_account_keys.len() {
            all_account_keys[ix.program_id_index].clone()
        } else {
            "UNKNOWN_PROGRAM_INDEX".to_string()
        };

        // Resolve Accounts
        let account_addresses: Vec<String> = ix.accounts.iter()
            .map(|&idx| {
                if idx < all_account_keys.len() {
                    all_account_keys[idx].clone()
                } else {
                    format!("UNKNOWN_IDX_{}", idx)
                }
            })
            .collect();

        ParsedInstruction {
            program_id,
            accounts: account_addresses,
            data: ix.data.clone(),
        }
    }).collect();

    ParsedTransaction {
        signature: tx.signatures[0].clone(),
        fee_payer: message.account_keys[0].clone(),
        is_success: meta.err.is_none(),
        account_keys: all_account_keys,
        instructions: parsed_instructions,
        log_messages: meta.log_messages,
        pre_balances: meta.pre_balances,
        post_balances: meta.post_balances,
        fee: meta.fee,
        compute_units_consumed: meta.compute_units_consumed,
    }
}

// ==========================================
// UTILITY FUNCTIONS
// ==========================================
pub fn load_from_json<T>(path: &str) -> Result<T, Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let parsed_data = serde_json::from_reader(reader)?;
    Ok(parsed_data)
}

fn print_transaction_summary(tx: &ParsedTransaction) {
    println!("--------------------------------");
    println!("Signature: {}", tx.signature);
    println!("Success:   {}", tx.is_success);
    println!("Fee Payer: {}", tx.fee_payer);
    println!("Fee:       {} lamports", tx.fee);
    if let Some(cu) = tx.compute_units_consumed {
        println!("Compute:   {} CU", cu);
    }
    println!("Total Accounts Resolved: {}", tx.account_keys.len());
    println!("--------------------------------");

    // Detect Raydium interactions
    for (index, ix) in tx.instructions.iter().enumerate() {
        if ix.program_id == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" {
            println!("Instruction #{}: Raydium Interaction Detected!", index);
            println!("  Data (Base58): {}", ix.data);
            println!("  -> Potential Swap instruction found.");
        }
    }
}

fn print_block_summary(block: &ParsedBlock) {
    println!("================================");
    println!("BLOCK SUMMARY");
    println!("================================");
    println!("Block Height:  {}", block.block_height);
    println!("Block Time:    {}", block.block_time);
    println!("Blockhash:     {}", block.blockhash);
    println!("Parent Slot:   {}", block.parent_slot);
    println!("Prev Hash:     {}", block.previous_blockhash);
    println!("Rewards:       {} entries", block.rewards.len());
    println!("Transactions:  {} total", block.transactions.len());
    println!("================================\n");

    // Print rewards
    if !block.rewards.is_empty() {
        println!("Rewards:");
        for reward in &block.rewards {
            println!("  {} - {} lamports ({})", 
                reward.pubkey, 
                reward.lamports, 
                reward.reward_type
            );
        }
        println!();
    }

    // Analyze transactions
    let successful = block.transactions.iter().filter(|tx| tx.is_success).count();
    let failed = block.transactions.len() - successful;
    let total_fees: u64 = block.transactions.iter().map(|tx| tx.fee).sum();

    println!("Transaction Stats:");
    println!("  Successful: {}", successful);
    println!("  Failed:     {}", failed);
    println!("  Total Fees: {} lamports", total_fees);
    println!();
}