extern crate core;

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::str::FromStr;
use bytes::Bytes;
use ethabi::Hash;
use glob::glob;
use rand::RngCore;
use revm::interpreter::{BytecodeLocked, CallInputs, Contract, CreateInputs, Gas, Host, InstructionResult, Interpreter, SelfDestructResult};
use revm::primitives::{B160, B256, Bytecode, Env, LatestSpec, Spec, U256};
use revm::primitives::ruint::aliases::{B1, B16};
use reqwest;
use revm::interpreter::analysis::to_analysed;
use serde_json::json;

pub struct TestHost {
    pub state: HashMap<B160, HashMap<U256, U256>>,
    pub prev_state: HashMap<B160, HashMap<U256, U256>>,
    pub call_traces: Vec<(B160, Bytes)>,
    pub erc20_affected: Vec<(B160, B160)>,
    pub env: Env,
    pub logs: HashMap<B160, Vec<(Vec<B256>, Bytes)>>,
    pub codes: HashMap<B160, Bytecode>,
    pub abis: HashMap<B160, ethabi::Contract>,
    pub context_mapping: HashMap<[u8; 4], String>,
    pub context_abi: ethabi::Contract,
    pub origin: B160,
    pub caller: B160,
    pub value: U256,
    pub data: Bytes,
    pub target: B160,
    pub inside_contract_call: bool,
}


pub fn get_rpc_url(name: String) -> &'static str {
    match name.as_str() {
        "ETH" => "https://eth.llamarpc.com",
        "BSC" => "https://bsc-dataseed.binance.org/",
        "BSC_TESTNET" => "https://data-seed-prebsc-1-s1.binance.org:8545/",
        "POLYGON" => "https://rpc-mainnet.maticvigil.com/",
        "MUMBAI" => "https://rpc-mumbai.maticvigil.com/",
        "ARBITRUM" => "https://arb1.arbitrum.io/rpc",
        _ => {
            panic!("Invalid chain type");
        }
    }
}

pub static mut RPC_URL: &str = "";




fn hex_encode_with_prefix(bytes: &[u8]) -> String {
    let mut hex = hex::encode(bytes);
    hex.insert_str(0, "0x");
    hex
}

pub fn to_str(address: B160) -> String {
    hex_encode_with_prefix(address.0.as_slice())
}

fn get_balance_rpc(address: B160) -> U256 {
    // call eth_getBalance
    let client = reqwest::blocking::Client::new();
    let url = unsafe { RPC_URL.to_string() };
    let response = client.post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [to_str(address), "latest"],
            "id": 1
        }))
        .send()
        .unwrap();
    let body = response.text().unwrap();
    let j = serde_json::from_str::<serde_json::Value>(&body).unwrap();
    let mut balance = j["result"].as_str().unwrap();
    balance = balance.trim_start_matches("0x");
    // println!("get_balance_rpc: {}", balance);
    U256::from_str_radix(balance, 16).unwrap()
}

fn get_block_hash(block: U256) -> B256 {
    let mut block_hex: String = hex::encode::<[u8; 32]>(block.to_be_bytes());
    block_hex.insert_str(0, "0x");
    // call eth_getBlockByNumber
    let client = reqwest::blocking::Client::new();
    let url = unsafe { RPC_URL.to_string() };
    let response = client.post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [block_hex, false],
            "id": 1
        }))
        .send()
        .unwrap();
    let body = response.text().unwrap();
    let j = serde_json::from_str::<serde_json::Value>(&body).unwrap();
    let hash = j["result"]["hash"].as_str().unwrap();
    // println!("get_block_hash: {}", hash);
    B256::from_slice(hex::decode(hash).unwrap().as_slice())
}

fn get_code_rpc(address: B160) -> Bytecode {
    // call eth_getCode
    let client = reqwest::blocking::Client::new();
    let url = unsafe { RPC_URL.to_string() };
    let response = client.post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_getCode",
            "params": [to_str(address), "latest"],
            "id": 1
        }))
        .send()
        .unwrap();
    let body = response.text().unwrap();
    let j = serde_json::from_str::<serde_json::Value>(&body).unwrap();
    let code = j["result"].as_str().unwrap();
    let code = code.trim_start_matches("0x");
    // println!("get_code_rpc: {} {}", address, code);
    to_analysed::<LatestSpec>(Bytecode::new_raw(Bytes::from(hex::decode(code).unwrap())))
}

fn get_storage_slot(address: B160, slot: U256) -> U256 {
    // call eth_getStorageAt
    let mut slot_hex: String = hex::encode::<[u8; 32]>(slot.to_be_bytes());
    // add prefix to slot_hex
    slot_hex.insert_str(0, "0x");
    let client = reqwest::blocking::Client::new();
    let url = unsafe { RPC_URL.to_string() };
    let response = client.post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_getStorageAt",
            "params": [to_str(address), slot_hex, "latest"],
            "id": 1
        }))
        .send()
        .unwrap();
    let body = response.text().unwrap();
    let j = serde_json::from_str::<serde_json::Value>(&body).unwrap();
    let slot_val = j["result"].as_str().unwrap();
    let slot_val = &slot_val[2..];
    // println!("get_storage_slot: {}@{}", slot_val, slot);
    U256::from_str_radix(slot_val, 16).unwrap()
}

impl Host for TestHost {
    fn step(&mut self, interpreter: &mut Interpreter, is_static: bool) -> InstructionResult {
        if unsafe {*interpreter.instruction_pointer} == 0xfd {
            println!("pc: {}@{:?} reverted", interpreter.program_counter(), interpreter.contract.address);
        }
        InstructionResult::Continue
    }

    fn step_end(&mut self, interpreter: &mut Interpreter, is_static: bool, ret: InstructionResult) -> InstructionResult {
        InstructionResult::Continue
    }

    fn env(&mut self) -> &mut Env {
        &mut self.env
    }

    fn load_account(&mut self, address: B160) -> Option<(bool, bool)> {
        Some((true, true))
    }

    fn block_hash(&mut self, number: U256) -> Option<B256> {
        Some(get_block_hash(number))
    }

    fn balance(&mut self, address: B160) -> Option<(U256, bool)> {
        Some((get_balance_rpc(address), true))
    }

    fn code(&mut self, address: B160) -> Option<(Bytecode, bool)> {
        match self.codes.get(&address) {
            Some(code) => Some((code.clone(), true)),
            None => {
                let code = get_code_rpc(address);
                self.codes.insert(address, code.clone());
                Some((code, true))
            }
        }
    }

    fn code_hash(&mut self, address: B160) -> Option<(B256, bool)> {
        unreachable!("code_hash should not be called")
    }

    fn sload(&mut self, address: B160, index: U256) -> Option<(U256, bool)> {
        match self.state.get_mut(&address) {
            Some(account) => {
                if let Some(value) = account.get(&index) {
                    return Some((*value, true))
                }
                let slot_val = get_storage_slot(address, index);
                account.insert(index, slot_val);
                return Some((slot_val, true))
            },
            None => {
                self.state.insert(address, HashMap::new());
                let slot_val = get_storage_slot(address, index);
                self.state.get_mut(&address).unwrap().insert(index, slot_val);
                return Some((slot_val, true))
            }
        }
    }

    fn sstore(&mut self, address: B160, index: U256, value: U256) -> Option<(U256, U256, U256, bool)> {
        self.state.get_mut(&address).unwrap().insert(index, value);
        Some((U256::ZERO, U256::ZERO, U256::ZERO, true))
    }

    fn log(&mut self, address: B160, topics: Vec<B256>, data: Bytes) {
        match self.logs.get_mut(&address) {
            Some(logs) => {
                logs.push((topics, data));
            },
            None => {
                self.logs.insert(address, vec![(topics, data)]);
            }
        }
    }

    fn selfdestruct(&mut self, address: B160, target: B160) -> Option<SelfDestructResult> {
        unreachable!("selfdestruct should not be called")
    }

    fn create(&mut self, inputs: &mut CreateInputs) -> (InstructionResult, Option<B160>, Gas, Bytes) {
        unreachable!("create should not be called")
    }

    fn call(&mut self, input: &mut CallInputs) -> (InstructionResult, Gas, Bytes) {
        macro_rules! handle_erc20 {
            ($data: expr, $target: expr) => {
                {
                    let data_slice = $data.as_slice();
                    match data_slice[0..4] {
                        // transfer
                        [0xa9, 0x05, 0x9c, 0xbb] => {
                            let dst = B160::from_slice(&data_slice[16..36]);
                            self.erc20_affected.push((dst, $target));
                        }
                        // transferFrom
                        [0x23, 0xb8, 0x72, 0xdd] => {
                            let src = B160::from_slice(&data_slice[12..32]);
                            let dst = B160::from_slice(&data_slice[48..68]);
                            self.erc20_affected.push((dst, $target));
                            self.erc20_affected.push((src, $target));
                        }
                        _ => {}
                    };
                }
            };
        }
        if input.context.address == B160::from_str("8891e33ba3c6A7b4E020A6180Eb07f4AED2d70CE").unwrap() {
            self.inside_contract_call = false;
            // println!("pc: {:?}@{:?}", input.input.to_vec(), input.context.address);
            let function_sig = &input.input.to_vec()[0..4];
            // println!("function_sig: {:?}", hex::encode(function_sig));

            let func_name = self.context_mapping.get(function_sig).unwrap();
            // println!("func_name: {:?}", func_name);
            let func = self.context_abi.function(func_name).unwrap();

            macro_rules! out_addr {
                ($addr: expr) => {
                    {
                        let mut out = [0u8; 32];
                        out[12..32].copy_from_slice(&$addr.0);
                        (InstructionResult::Continue, Gas::new(u64::MAX), Bytes::from(out.to_vec()))
                    }
                };
            }



            match func_name.as_str() {
                "get_caller" => {
                    // convert b160 address to bytes
                    return out_addr!(self.caller);
                },
                "get_target" => {
                    return out_addr!(self.target);
                },
                "get_value" => {
                    let mut out = [0u8; 32];
                    out.copy_from_slice(&self.value.to_be_bytes::<32>());
                    return (InstructionResult::Continue, Gas::new(u64::MAX), Bytes::from(out.to_vec()));
                },
                "get_data" => {
                    return (InstructionResult::Continue, Gas::new(u64::MAX), self.data.clone());
                },
                "get_affected_contracts" => {
                    // println!("get_affected_contracts: {:?}", self.call_traces);
                    let encoded = ethabi::encode(&[
                        ethabi::Token::Array(self.call_traces.iter().map(|(addr, _)| ethabi::Token::Address({
                            ethabi::Address::from_slice(&addr.0)
                        })).collect()),
                        ethabi::Token::Array(self.call_traces.iter().map(|(_, data)| ethabi::Token::Bytes(
                            ethabi::Bytes::from(data.to_vec())
                        )).collect()),
                    ]).to_vec();
                    // println!("get_affected_contracts: {:?}", hex::encode(&encoded));
                    return (InstructionResult::Continue, Gas::new(u64::MAX), Bytes::from(encoded));
                },
                "get_affected_accounts_ierc20" => {
                    let encoded = ethabi::encode(&[
                        ethabi::Token::Array(self.erc20_affected.iter().map(|(addr, _)| ethabi::Token::Address({
                            ethabi::Address::from_slice(&addr.0)
                        })).collect()),
                        ethabi::Token::Array(self.erc20_affected.iter().map(|(_, addr)| ethabi::Token::Address({
                            ethabi::Address::from_slice(&addr.0)
                        })).collect()),
                    ]).to_vec();
                    return (InstructionResult::Continue, Gas::new(u64::MAX), Bytes::from(encoded));
                },
                "call_prev_state" => {
                    let input = self.context_abi.function("call_prev_state")
                        .unwrap()
                        .decode_input(&input.input.to_vec()[4..]).unwrap();
                    let target = if let ethabi::Token::Address(x) = input[0]
                    { B160::from(x.0) } else { panic!("invalid target") };
                    let caller = if let ethabi::Token::Address(x) = input[1]
                    { B160::from(x.0) } else { panic!("invalid caller") };
                    let data = if let ethabi::Token::Bytes(x) = input[2].clone()
                    { Bytes::from(x.to_vec()) } else { panic!("invalid data") };
                    let value: U256 = if let ethabi::Token::Uint(x) = input[3]
                    { U256::from_str(x.to_string().as_str()).unwrap() } else { panic!("invalid value") };
                    let temp = self.state.clone();
                    self.state = self.prev_state.clone();
                    let (ret, res) = call_func(
                        self, caller, target, data, value,
                    );
                    let encoded_res = ethabi::encode(
                        &[ethabi::Token::Bytes(ethabi::Bytes::from(res.to_vec()))]
                    ).to_vec();
                    self.state = temp;
                    return (ret, Gas::new(u64::MAX), Bytes::from(encoded_res));

                },
                "test_call" => {
                    self.data = input.input.clone();
                    self.value = input.context.apparent_value;
                    self.caller = input.context.caller;
                    self.target = input.context.address;
                    self.inside_contract_call = true;
                    self.prev_state = self.state.clone();
                    self.call_traces.clear();
                    self.erc20_affected.clear();

                    // do call
                    let input = self.context_abi.function("test_call")
                        .unwrap()
                        .decode_input(&input.input.to_vec()[4..]).unwrap();
                    let target = if let ethabi::Token::Address(x) = input[0]
                        { B160::from(x.0) } else { panic!("invalid target") };
                    let caller = if let ethabi::Token::Address(x) = input[1]
                        { B160::from(x.0) } else { panic!("invalid caller") };
                    let data = if let ethabi::Token::Bytes(x) = input[2].clone()
                        { Bytes::from(x.to_vec()) } else { panic!("invalid data") };
                    let value: U256 = if let ethabi::Token::Uint(x) = input[3]
                        { U256::from_str(x.to_string().as_str()).unwrap() } else { panic!("invalid value") };

                    // erc20 analysis
                    let input_vec = data.to_vec();
                    handle_erc20!(input_vec, target);

                    let (ret, res) = call_func(
                        self, caller, target, data, value,
                    );
                    let encoded_res = ethabi::encode(
                        &[ethabi::Token::Bytes(ethabi::Bytes::from(res.to_vec()))]
                    ).to_vec();
                    self.inside_contract_call = false;
                    return (ret, Gas::new(u64::MAX), Bytes::from(encoded_res));
                },
                _ => {
                    panic!("unknown function")
                }
            }
        }

        if self.inside_contract_call {
            self.call_traces.push((input.context.address, input.input.clone()));
            // erc20 analysis
            let data = input.input.to_vec();
            handle_erc20!(data, input.context.address);
        }

        let code = match self.codes.get(&input.context.code_address) {
            Some(code) => {
                code.clone()
            }
            None => {
                let code = get_code_rpc(input.context.code_address);
                self.codes.insert(input.context.code_address, code.clone());
                code
            }
        };
        let contract = Contract {
            input: input.input.clone(),
            bytecode: BytecodeLocked::try_from(code).unwrap(),
            address: input.context.address,
            caller: input.context.caller,
            value: input.context.apparent_value,
        };
        let mut interpreter = Interpreter::new(contract, u64::MAX, input.is_static);
        let ret = interpreter.run::<TestHost, LatestSpec>(self);
        self.inside_contract_call = false;
        (ret, Gas::new(u64::MAX), interpreter.return_value())
    }
}

fn generate_random_address() -> B160 {
    let mut rng = rand::thread_rng();
    let mut address = [0u8; 20];
    rng.fill_bytes(&mut address);
    B160::from(address)
}


fn call_func(host: &mut TestHost, caller: B160, target: B160, data: Bytes, value: U256) -> (InstructionResult, Bytes) {
    host.origin = target;
    host.logs.clear();
    let code = match host.codes.get(&target) {
        None => {
            let code = get_code_rpc(target);
            host.codes.insert(target, code.clone());
            code
        }
        Some(code) => {
            code.clone()
        }
    };

    let contract = Contract {
        input: data,
        bytecode: BytecodeLocked::try_from(code).unwrap(),
        address: target,
        caller,
        value,
    };
    let mut interpreter = Interpreter::new(contract, u64::MAX, false);
    let ret = interpreter.run_inspect::<TestHost, LatestSpec>(host);
    return (ret, interpreter.return_value());
}


fn load_context_abi() -> (HashMap<[u8; 4], String>, ethabi::Contract) {
    let mut result = HashMap::new();
    let path = "./Context.abi";
    let file = File::open(path).unwrap();
    let abis = ethabi::Contract::load(file).unwrap();
    for func in abis.functions() {
        let sig = func.short_signature();
        unsafe {
            result.insert(sig, func.name.to_string());
        }
    }
    (result, abis)
}


fn main() {
    let mut args = env::args();
    if args.len() < 3 {
        println!("Usage: {} <glob> <chain>", args.nth(0).unwrap());
        return;
    }


    let (context_info, context_abi) = load_context_abi();
    // println!("{:?}", context_info);
    args.next();
    let path = args.next().unwrap();
    let chain = args.next().unwrap();
    println!("path: {}, chain: {}", path, chain);
    // glob pattern

    unsafe {
        RPC_URL = get_rpc_url(chain);
    }
    let mut invariant_deployed_addresses = Vec::new();
    let mut host = TestHost {
        state: Default::default(),
        prev_state: Default::default(),
        call_traces: vec![],
        erc20_affected: Default::default(),
        env: Default::default(),
        logs: Default::default(),
        codes: Default::default(),
        abis: Default::default(),
        context_mapping: context_info,
        context_abi,
        origin: Default::default(),
        caller: Default::default(),
        value: Default::default(),
        data: Default::default(),
        target: Default::default(),
        inside_contract_call: false,
    };

    let mut name_to_abi = HashMap::new();
    let mut name_to_address = HashMap::new();

    for entry in glob(path.as_str()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                // println!("{:?}", path);
                let seg = path.to_str().unwrap();
                if seg.ends_with(".abi") {
                    name_to_abi.insert(
                        seg[0..seg.len() - 4].to_string(),
                        ethabi::Contract::load(File::open(path.clone()).unwrap()).unwrap()
                    );
                    let mut file = File::open(path).unwrap();

                    let mut contents = String::new();
                    file.read_to_string(&mut contents).unwrap();
                    let json: serde_json::Value = serde_json::from_str(&contents).unwrap();
                    // abi file
                    for func in json.as_array().unwrap() {
                        let obj = func.as_object().unwrap();
                        if obj.get("type").unwrap().as_str().unwrap() != "function" {
                            continue;
                        }
                        let name = obj.get("name").unwrap().as_str().unwrap();
                    }
                } else if seg.ends_with(".bin") {
                    let deploy_address = generate_random_address();

                    // remove seg suffix
                    name_to_address.insert(
                        seg[0..seg.len() - 4].to_string(),
                        deploy_address
                    );

                    let mut file = File::open(path).unwrap();
                    let mut contents = String::new();
                    file.read_to_string(&mut contents).unwrap();
                    let mut contract_code = hex::decode(contents.trim()).unwrap();
                    let bytes = Bytes::from(contract_code);
                    let bytecode = BytecodeLocked::try_from(
                        to_analysed::<LatestSpec>(Bytecode::new_raw(bytes))
                    ).unwrap();

                    let contract = Contract {
                        input: Bytes::new(),
                        bytecode,
                        address: deploy_address,
                        caller: generate_random_address(),
                        value: U256::ZERO,
                    };
                    let mut interpreter = Interpreter::new(contract, u64::MAX, false);
                    let ret = interpreter.run_inspect::<TestHost, LatestSpec>(&mut host);
                    assert_ne!(ret, InstructionResult::Revert);
                    invariant_deployed_addresses.push(deploy_address);
                    host.codes.insert(deploy_address, to_analysed::<LatestSpec>(
                        Bytecode::new_raw(interpreter.return_value())
                    ));
                    println!("deployed address: {:?}", deploy_address);
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }

    for (name, abi) in name_to_abi {
        let address = name_to_address.get(&name).unwrap();
        host.abis.insert(*address, abi);
    }

    for addr in invariant_deployed_addresses {
        let abi = host.abis.get(&addr).unwrap().clone();
        for (name, _) in &abi.functions {

            if name.starts_with("test_") {
                let data = abi.function(name.as_str()).unwrap().encode_input(&[]).unwrap().to_vec();
                let (ret, res) = call_func(&mut host, generate_random_address(),
                                           addr.clone(), Bytes::from(data), U256::ZERO);
                println!("calling test_{:?} @ {:?}, ret: {:?}, res: {:?}", name, addr, ret, res);
            }
        }
    }

}


