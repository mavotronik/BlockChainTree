use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use crate::Tools;
use crate::merkletree::MerkleTree;
use crate::Transaction::Transaction;
use crate::Token;
use base64;
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::transmute;
use std::mem::transmute_copy;
use crate::DumpHeaders::Headers;


#[macro_export]
macro_rules! bytes_to_u64  {
    ($buffer:expr,$buffer_index:expr) => {
       (&$buffer[$buffer_index..$buffer_index+8]).read_u64::<BigEndian>().unwrap()
    };
}

static ALREADY_SET:&str = "data is already set";

#[derive(Debug)]
pub struct BasicInfo{
    timestamp:u64,
    PoW:BigUint,
    previous_hash:[u8;32],
    current_hash:[u8;32],
    height:u64,
    difficulty:[u8;32]
}


impl BasicInfo{
    pub fn new(//miner:[u8;33],
                timestamp:u64,
                PoW:BigUint,
                previous_hash:[u8;32],
                current_hash:[u8;32],
                height:u64,
                difficulty:[u8;32]) -> BasicInfo{
        return BasicInfo{//miner:miner,
                        timestamp:timestamp,
                        PoW:PoW,
                        previous_hash:previous_hash,
                        current_hash:current_hash,
                        height:height,
                        difficulty:difficulty};
    }

    pub fn get_dump_size(&self) -> usize{
        let to_return = 8
                    + Tools::bigint_size(&self.PoW)
                    + 32
                    + 32
                    + 8
                    + 32;
        return to_return;
    }
    pub fn dump(&self,buffer:&mut Vec<u8>) -> Result<(),&'static str>{

        // dumping timestamp
        for byte in self.timestamp.to_be_bytes().iter(){
            buffer.push(*byte);
        }

        // dumping previous hash
        for byte in self.previous_hash.iter(){
            buffer.push(*byte);
        }

        // dumping current hash
        for byte in self.current_hash.iter(){
            buffer.push(*byte);
        }

        // dumping height
        for byte in self.height.to_be_bytes().iter(){
            buffer.push(*byte);
        }

        // dumping difficulty
        buffer.extend(self.difficulty);

        // dumping PoW
        let result = Tools::dump_biguint(&self.PoW, buffer);
        if result.is_err(){
            return Err("could not dump PoW");
        }

        return Ok(());
    }

    pub fn parse(data:&[u8]) -> Result<BasicInfo,&'static str>{
        let mut index:usize = 0;

        if data.len() <= 112{
            return Err("Not enough data to parse");
        }

        // parsing timestamp
        let timestamp = bytes_to_u64!(data,index);
        index += 8;

        // parsing previous hash
        let previous_hash:[u8;32] = unsafe{transmute_copy(&data[index])};
        index += 32;

        // parsing current hash
        let current_hash:[u8;32] = unsafe{transmute_copy(&data[index])};
        index += 32;

        // parsing height
        let height:u64 = bytes_to_u64!(data,index);
        index += 8;

        // parsing difficulty
        let difficulty:[u8;32] = unsafe{transmute_copy(&data[index])};
        index += 32;
        
        // parsing PoW
        let result = Tools::load_biguint(&data[index..]);
        if result.is_err(){
            return Err("Error loading PoW");
        }
        let PoW: BigUint;
        match result{
            Err(e)=>{return Err(e);}
            Ok(a) => {PoW = a.0;}
        }

        return Ok(BasicInfo{timestamp:timestamp,
                        PoW:PoW,
                        previous_hash:previous_hash,
                        current_hash:current_hash,
                        height:height,
                        difficulty:difficulty});
    } 
}

#[derive(Debug)]
pub struct TransactionToken{
    transaction:Option<Transaction>,
    token:Option<Token::TokenAction>
}
impl TransactionToken{
    pub fn new(tr:Option<Transaction>,tk:Option<Token::TokenAction>)->TransactionToken{
        return TransactionToken{transaction:tr,
                                token:tk};
    }
    pub fn is_empty(&self) -> bool{
        return self.transaction.is_none() && self.token.is_none();  
    }

    pub fn is_transaction(&self) -> bool{
        return !self.transaction.is_none();
    }
    pub fn is_token(&self) -> bool{
        return !self.token.is_none();
    }

    pub fn set_transaction(&mut self, 
                            transaction:Transaction) 
                            -> Result<(),&'static str>{
        if !self.is_empty(){
            return Err(ALREADY_SET);
        }

        self.transaction = Some(transaction);

        return Ok(());
    }
    pub fn set_token(&mut self, token:Token::TokenAction) 
                            -> Result<(),&'static str>{
        if !self.is_empty(){
            return Err(ALREADY_SET);
        }

        self.token = Some(token);

        return Ok(());
    }

    pub fn get_transaction(&self) -> &Option<Transaction>{
        return &self.transaction;
    }
    pub fn get_token(&self) -> &Option<Token::TokenAction>{
        return &self.token;
    }
    pub fn get_hash(&self,previous_hash:&[u8;32]) -> Box<[u8;32]>{
        if self.is_transaction(){
            return self.transaction.as_ref().unwrap().hash(previous_hash);
        }else{
            return self.token.as_ref().unwrap().hash(previous_hash);
        }
    }
    pub fn get_dump_size(&self) -> usize{
        if self.is_transaction(){
            return self.transaction.as_ref().unwrap().get_dump_size();
        }
        else{
            return self.token.as_ref().unwrap().get_dump_size();
        }
    }
    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        if self.is_transaction(){
            return self.transaction.as_ref().unwrap().dump();
        }else{
            return self.token.as_ref().unwrap().dump();
        }
    }   
}

#[derive(Debug)]
pub struct TransactionBlock{
    transactions:Vec<TransactionToken>,
    fee:BigUint,
    merkle_tree:Option<MerkleTree>,
    merkle_tree_root:[u8;32],
    default_info:BasicInfo
}

impl TransactionBlock{
    pub fn new(transactions:Vec<TransactionToken>,
                fee:BigUint,
                default_info:BasicInfo,
                merkle_tree_root:[u8;32]) -> TransactionBlock{
        return TransactionBlock{transactions:transactions,
                                fee:fee,merkle_tree:None,
                                default_info:default_info,
                                merkle_tree_root:merkle_tree_root};
    }

    pub fn merkle_tree_is_built(&self) -> bool{
        return !self.merkle_tree.is_none();
    }

    pub fn build_merkle_tree(&mut self) ->Result<(),&'static str>{
        let mut new_merkle_tree = MerkleTree::new();
        let mut hashes:Vec<&[u8;32]> = Vec::with_capacity(self.transactions.len());

        for TT in self.transactions.iter(){
            let res = TT.get_hash(&self.default_info.previous_hash);
            hashes.push(Box::leak(res));      
        }

        let res = new_merkle_tree.add_objects(hashes);
        if !res{
            return Err("Error adding objects to the merkle tree");
        }
        self.merkle_tree = Some(new_merkle_tree);
        return Ok(());
    }

    pub fn check_merkle_tree(&mut self) -> Result<bool,&'static str>{
        // build merkle tree if not built
        if !self.merkle_tree_is_built(){
            let res = self.build_merkle_tree();
            if res.is_err(){
                return Err(res.err().unwrap());
            }
        }

        // transmute computed root into 4 u64 bytes 
        let constructed_tree_root_raw = self.merkle_tree.as_ref().unwrap().get_root(); 
        let constructed_tree_root_raw_root:&[u64;4] = unsafe{
                                transmute(constructed_tree_root_raw)};
        
        // transmute root into 4 u64 bytes 
        let root:&[u64;4] = unsafe{transmute(&self.merkle_tree_root)};

        for (a,b) in root.iter().zip(
                            constructed_tree_root_raw_root.iter()){
            if *a != *b{
                return Ok(false);
            }
        }
        return Ok(true);
    }

    pub fn get_dump_size(&self) -> usize{
        let mut size:usize = 1;
        for transaction in self.transactions.iter(){
            size += transaction.get_dump_size();
        }
        size += Tools::bigint_size(&self.fee);
        size += 32;
        size += self.default_info.get_dump_size();

        return size;
    }

    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        let size:usize = self.get_dump_size();

        let mut to_return:Vec<u8> = Vec::with_capacity(size);

        //header
        to_return.push(Headers::TransactionBlock as u8);

        // merkle tree root
        to_return.extend(self.merkle_tree_root.iter());

        // default info
        let result = self.default_info.dump(&mut to_return);
        if result.is_err(){
            return Err("Error dumping default info");
        }

        // fee
        let result = Tools::dump_biguint(&self.fee, &mut to_return);
        if result.is_err(){
            return Err("Error dumping BigUInt");
        }
        
        // amount of transactions
        let amount_of_transactions:u16;
        if self.transactions.len() > 0xFFFF{
            return Err("Too much transactions");
        }else{
            amount_of_transactions = self.transactions.len() as u16
        }

        to_return.extend(amount_of_transactions.to_be_bytes().iter());

        // transactions/tokens
        for transaction in self.transactions.iter(){
            // size of transaction
            let size_of_transaction:u32 = transaction.get_dump_size() as u32;
            to_return.extend(size_of_transaction.to_be_bytes().iter());

            for byte in transaction.dump().unwrap().iter(){
                to_return.push(*byte);
            }
        }

        return Ok(to_return);
    }

    pub fn parse(data:&[u8],block_size:u32) -> Result<TransactionBlock,&'static str>{
        let mut offset:usize = 0;

        // merkle tree root
        let merkle_tree_root:[u8;32] = data[..32].try_into().unwrap();
        offset += 32; // inc offset

        
        // default info
        let result = BasicInfo::parse(&data[offset..]);
        if result.is_err(){
            return Err("Bad BasicInfo");
        }
        let default_info:BasicInfo = result.unwrap();
        offset += default_info.get_dump_size(); // inc offset

        // fee
        let result = Tools::load_biguint(&data[offset..]);
        if result.is_err(){
            return Err("Error in parsing fee");
        }
        let result_enum = result.unwrap();
        let fee = result_enum.0;

        offset += result_enum.1; // inc offset

        // transactions
        let amount_of_transactions:u16 = u16::from_be_bytes(
                                data[offset..offset+2].try_into().unwrap());
        offset += 2; // inc offset
        
        let mut transactions:Vec<TransactionToken> = Vec::with_capacity(amount_of_transactions as usize);

        for _ in 0..amount_of_transactions{
            let transaction_size:u32 = u32::from_be_bytes(data[offset..offset+4].try_into().unwrap())-1;
            
            offset += 4; // inc offset

            let trtk_type:u8 = data[offset];
            offset += 1;

            let mut trtk:TransactionToken = TransactionToken::new(None,None);

            if trtk_type == Headers::Transaction as u8{
                // if transaction
                let result = Transaction::parse_transaction(
                    &data[offset..offset+(transaction_size as usize)],transaction_size as u64);
                if result.is_err(){
                    return Err("Error parsing transaction");
                }
                let transaction = result.unwrap();
                let result = trtk.set_transaction(transaction);
                if result.is_err(){
                    return Err("Error setting transaction");
                }
            } else if trtk_type == Headers::Token as u8{
                // if token action
                let result = Token::TokenAction::parse(
                    &data[offset..offset+(transaction_size as usize)],transaction_size as u64);
                if result.is_err(){
                    return Err("Error parsing token");
                }
                let token = result.unwrap();
                let result = trtk.set_token(token);
                if result.is_err(){
                    return Err("Error setting token");
                }  
            }else{
                return Err("Not existant type");
            }
            offset += transaction_size as usize; // inc offset
            
            transactions.push(trtk); 
        }

        if offset != block_size as usize{
            return Err("Could not parse block");
        }

        let transaction_block = TransactionBlock::new(transactions,
                                            fee,
                                            default_info,
                                            merkle_tree_root);

        return Ok(transaction_block);

    }

    pub fn hash(&self) -> Result<[u8;32],&'static str>{
        let dump:Vec<u8> = self.dump().unwrap();

        return Ok(Tools::hash(&dump));
    }

}

pub struct TokenBlock{
    pub default_info:BasicInfo,
    pub token_signature:String,
    pub payment_transaction:Transaction
}

impl TokenBlock{
    pub fn new(default_info:BasicInfo,
                token_signature:String,
                payment_transaction:Transaction) -> TokenBlock{

        return TokenBlock{default_info:default_info,
                        token_signature:token_signature,
                        payment_transaction:payment_transaction}
    }

    pub fn get_dump_size(&self) -> usize{
        return self.default_info.get_dump_size()
                +self.token_signature.len()
                +1
                +self.payment_transaction.get_dump_size();
    }

    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        let dump_size:usize = self.get_dump_size();
        
        let mut dump:Vec<u8> = Vec::with_capacity(dump_size);

        // header
        dump.push(Headers::TokenBlock as u8);

        // // dumping token signature
        // for byte in self.token_signature.as_bytes().iter(){
        //     dump.push(*byte);
        // }
        // dump.push(0);

        // dumping payment transaction
        let transaction_len:u32 = self.payment_transaction.get_dump_size() as u32;
        dump.extend(transaction_len.to_be_bytes().iter());

        let result = self.payment_transaction.dump();
        if result.is_err(){
            return Err("Error dumping payment transaction");
        }
        dump.extend(result.unwrap());

        // dumping default info
        let result = self.default_info.dump(&mut dump);
        if result.is_err(){
            return Err("Error dumping default info");
        }

        return Ok(dump);
    }

    pub fn parse(data:&[u8],block_size:u32) -> Result<TokenBlock,&'static str>{
        
        let mut offset:usize = 0;

        // parsing token signature
        let mut token_signature:String = String::new();
        // for byte in data{
        //     offset += 1;
        //     if *byte == 0{
        //         break;
        //     }
        //     token_signature.push(*byte as char);
        // }

        // parsing transaction
        let transaction_size:u32 = u32::from_be_bytes(data[offset..offset+4].try_into().unwrap());
        offset += 4;
        
        if data[offset] != Headers::Transaction as u8{
            return Err("Transaction wasn't found");
        }
        offset += 1;
        let result = Transaction::parse_transaction(&data[offset..offset+transaction_size as usize], (transaction_size-1) as u64);
        if result.is_err(){
            return Err("Parsing transaction error");
        }

        let transaction = result.unwrap();
        offset += (transaction_size-1) as usize;

        // parsing basic info 
        let result = BasicInfo::parse(&data[offset..block_size as usize]);
        if result.is_err(){
            return Err("Parsing basic info error");
        }
        let default_info = result.unwrap();

        offset += default_info.get_dump_size();

        if offset != block_size as usize{
            return Err("Error parsing token block");
        }


        return Ok(TokenBlock{default_info:default_info,
                            token_signature:token_signature,
                            payment_transaction:transaction});
    }

    pub fn hash(&self) -> Result<[u8;32],&'static str>{
        let dump:Vec<u8> = self.dump().unwrap();

        return Ok(Tools::hash(&dump));
    }

}



pub struct SummarizeBlock{
    default_info:BasicInfo,
    founder_transaction:Transaction

}

impl SummarizeBlock{
    pub fn new(default_info:BasicInfo,
                founder_transaction:Transaction) -> SummarizeBlock{

        return SummarizeBlock{default_info:default_info,
                    founder_transaction:founder_transaction};
    }

    pub fn get_dump_size(&self) -> usize{
        return 1 // header
                +self.default_info.get_dump_size()
                +self.founder_transaction.get_dump_size();
    }
    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{

        let mut to_return:Vec<u8> = Vec::with_capacity(self.get_dump_size());

        // header
        to_return.push(Headers::SummarizeBlock as u8);

        // dump transaction
        let result = self.founder_transaction.dump();
        if result.is_err(){
            return Err(result.err().unwrap());
        }
        let mut transaction_dump = result.unwrap();
        to_return.extend((transaction_dump.len() as u64).to_be_bytes());
        to_return.append(&mut transaction_dump);

        // dump basic info
        let result = self.default_info.dump(&mut to_return);
        if result.is_err(){
            return Err(result.err().unwrap());
        }

        return Ok(to_return);
    }

    pub fn parse(data:&[u8]) -> Result<SummarizeBlock,&'static str>{
        if data.len() <= 8{
            return Err("Not enough data");
        }
        let mut offset:usize = 0;
        
        // parse transaction
        let transaction_size:usize = u64::from_be_bytes(data[0..8].try_into().unwrap()) as usize;
        offset += 8;
        if data.len()<transaction_size+8{
            return Err("Error while parsing transaction");
        }
        let result = Transaction::parse_transaction(&data[offset..offset+transaction_size], transaction_size as u64);
        if result.is_err(){
            return Err(result.err().unwrap());
        }
        let transaction = result.unwrap();
        offset += transaction_size;

        // parse default info
        let result = BasicInfo::parse(&data[offset..]);
        if result.is_err(){
            return Err(result.err().unwrap());
        }
        let default_info = result.unwrap();

        return Ok(SummarizeBlock{default_info:default_info,
                        founder_transaction:transaction});

    }

    pub fn hash(&self) -> Result<[u8;32],&'static str>{
        let result = self.dump();

        if result.is_err(){
            return Err(result.err().unwrap());
        }
        let dump:Vec<u8>; 
        unsafe{dump = result.unwrap_unchecked();};

        return Ok(Tools::hash(&dump));
    }

}


pub struct SumTransactionBlock{
    transaction_block: Option<TransactionBlock>,
    summarize_block: Option<SummarizeBlock>
}

impl SumTransactionBlock{
    pub fn new(transaction_block:Option<TransactionBlock>,
                summarize_block:Option<SummarizeBlock>)
                ->SumTransactionBlock{
                                       
        return SumTransactionBlock{transaction_block:transaction_block,
                                summarize_block:summarize_block};
    }
    
    pub fn is_empty(&self) -> bool{
        return self.summarize_block.is_none() && 
                self.transaction_block.is_none();
    }

    pub fn is_transaction_block(&self) -> bool{
        return self.transaction_block.is_none();
    }
    pub fn is_summarize_block(&self) -> bool{
        return self.summarize_block.is_none();
    }
    pub fn hash(&self) -> Result<[u8;32],&'static str>{
        if self.is_transaction_block(){
            return self.transaction_block.as_ref().unwrap().hash();
        }else{
            return self.summarize_block.as_ref().unwrap().hash();
        }
    }

    pub fn get_dump_size(&self) -> usize{
        if self.is_transaction_block(){
            return self.transaction_block.as_ref().unwrap().get_dump_size();
        }else{
            return self.summarize_block.as_ref().unwrap().get_dump_size();
        }
    }

    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        if self.is_transaction_block(){
            return self.transaction_block.as_ref().unwrap().dump();
        }else{
            return self.summarize_block.as_ref().unwrap().dump();
        }
    }
}

