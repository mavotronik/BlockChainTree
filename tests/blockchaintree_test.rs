use blockchaintree::block::{self, BasicInfo};
use blockchaintree::{self, transaction::Transactionable};
use num_bigint::ToBigUint;
use std::sync::Arc;

static SENDER: &[u8; 33] = b"123456789012345678901234567890123";
static RECIEVER: &[u8; 33] = b"123456789012345678901234567890123";
static SIGNATURE: &[u8; 64] = b"1234567890123456789012345678901234567890123456789012345678901234";
static PREV_HASH: &[u8; 32] = b"12345678901234567890123456789012";

#[tokio::test]
async fn chain_test() {
    let mut blockchain = blockchaintree::blockchaintree::BlockChainTree::without_config().unwrap();

    let default_info = BasicInfo::new(
        500,
        1000u64.to_biguint().unwrap(),
        [0u8; 32],
        [1u8; 32],
        0,
        [5u8; 32],
    );
    let tr = blockchaintree::transaction::Transaction::new(
        SENDER,
        RECIEVER,
        121212,
        SIGNATURE,
        2222222288u64.to_biguint().unwrap(),
    );

    let block = block::TokenBlock::new(default_info, String::new(), tr);

    let derivative_chain = if let chain = blockchain
        .get_derivative_chain(SENDER)
        .await
        .unwrap()
        .unwrap()
        .clone()
    {
        chain
    } else {
        blockchain
            .create_derivative_chain(SENDER, PREV_HASH, 0)
            .await
            .unwrap()
            .clone()
    };

    derivative_chain
        .write()
        .await
        .add_block(&block)
        .await
        .unwrap();

    let block_db = derivative_chain
        .read()
        .await
        .find_by_height(0)
        .unwrap()
        .unwrap();
    assert_eq!(block_db.payment_transaction.get_sender(), SENDER);
}
