use std::str::FromStr;
use bytes::Bytes;
use revm::primitives::{B160, U256};


pub static INCH_API: &str = "https://api.1inch.exchange/v5.0/";

pub fn get_router_and_weth(network: &str) -> (B160, B160, u8) {
    match network {
        "ETH" => {
            (B160::from_str("0x7a250d5630b4cf539739df2c5dacb4c659f2488d").unwrap(), B160::from_str("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap(), 1)
        }
        "BSC" => {
            (B160::from_str("0x05ff2b0db69458a0750badebc4f9e13add608c7f").unwrap(), B160::from_str("0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c").unwrap(), 56)
        },
        "POLYGON" => {
            (B160::from_str("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506").unwrap(), B160::from_str("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270").unwrap(), 137)
        },
        _ => {
            panic!("Unsupported network: {} for buying / liquidation", network);
        }
    }
}


fn get_1inch_swap_args(from_token: B160, to_token: B160, amount: U256,caller: B160, network: &str) -> (U256, B160, Bytes) {
    let (_,_,chain_id) = get_router_and_weth(network);
    let result = reqwest::blocking::get(
        format!(
            "{}{}/swap?fromTokenAddress={:?}&toTokenAddress={:?}&amount={}&disableEstimate=true&slippage=50&fromAddress={:?}",
            INCH_API,
            chain_id,
            from_token,
            to_token,
            amount.to_string(),
            caller
        )
    ).unwrap();
    let json: serde_json::Value = result.json().unwrap();

    let tx = json["tx"].as_object().unwrap();
    let to = B160::from_str(tx.get("to").unwrap().as_str().unwrap()).unwrap();
    let data_hex = tx.get("data").unwrap().as_str().unwrap().strip_prefix("0x").unwrap();
    let data_by = hex::decode(data_hex).unwrap();
    let data = Bytes::from(
        data_by
    );
    let value = U256::from_str(tx.get("value").unwrap().as_str().unwrap()).unwrap();
    return (value, to, data);
}


fn find_best_path_1nch(from_token: B160, to_token: B160, amount: U256, liquidation_src: String) -> Vec<B160> {
    let result = reqwest::blocking::get(
        format!(
            "{}1/quote?fromTokenAddress={:?}&toTokenAddress={:?}&amount={}&protocols={}",
            INCH_API,
            from_token,
            to_token,
            amount.to_string(),
            liquidation_src
        )
    ).unwrap();
    println!("{:?}", format!(
        "{}1/quote?fromTokenAddress={:?}&toTokenAddress={:?}&amount={}&protocols={}",
        INCH_API,
        from_token,
        to_token,
        amount.to_string(),
        liquidation_src
    ));
    let json: serde_json::Value = result.json().unwrap();


    let protocols = json["protocols"].as_array().unwrap();
    // find shortest path
    let mut path = vec![];
    let mut current_min_len = 100000;
    assert!(protocols.len() > 0, "Cannot find swap path for {:?} -> {:?}", from_token, to_token);
    for protocol in protocols {
        let protocol_arr = protocol.as_array().unwrap();
        let proper = protocol_arr.iter().map(|x| {
            x.as_array().unwrap().len() == 1
        }).all(|x| x);
        if !proper {
            continue;
        }
        if protocol_arr.len() < current_min_len {
            current_min_len = protocol_arr.len();
            path = protocol_arr.clone();
        }
    }

    assert!(path.len() > 0, "Cannot find proper swap path for {:?} -> {:?}", from_token, to_token);

    macro_rules! get_token {
        ($i: expr, $k: expr) => {
            B160::from_str(path[$i].as_array().unwrap()[0].as_object().unwrap().get($k).unwrap().as_str().unwrap()).unwrap()
        };
    }

    let mut token_path = vec![
        get_token!(0, "fromTokenAddress"),
    ];

    for i in 0..path.len() {
        token_path.push(get_token!(i, "toTokenAddress"));
    }

    return token_path;
}


pub fn buy_token(
    token: B160,
    amount: U256,
    caller: B160,
    network: &str,
) -> (U256, B160, Bytes) {
    let (_, weth, _) = get_router_and_weth(network);
    if token == weth {
        // directly deposit
        return (amount, weth, Bytes::from(vec![0xd0, 0xe3, 0x0d, 0xb0]));
    }
    return get_1inch_swap_args(
        B160::from_str("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee").unwrap(),
        token,
        amount,
        caller,
        network
    );
    // let path = find_best_path_1nch(
    //     weth,
    //     token,
    //     amount,
    //     "UNISWAP_V2".to_string()
    // );
    // assert_eq!(path[0], weth, "First token in path should be WETH");
    //
    // let path_abi = path.iter().map(
    //     |x| {
    //         ethabi::token::Token::Address(
    //             ethabi::Address::from_slice(&x.0)
    //         )
    //     }
    // ).collect::<Vec<_>>();
    //
    // // function swapExactETHForTokensSupportingFeeOnTransferTokens(
    // //     uint amountOutMin,
    // //     address[] calldata path,
    // //     address to,
    // //     uint deadline
    // // )
    // let abi_bys = ethabi::encode(
    //     &[
    //         ethabi::token::Token::Uint(
    //             ethabi::Uint::from(0)
    //         ),
    //         ethabi::token::Token::Array(path_abi),
    //         ethabi::token::Token::Address(
    //             ethabi::Address::from_slice(&caller.0)
    //         ),
    //         ethabi::token::Token::Uint(
    //             ethabi::Uint::MAX
    //         ),
    //     ]
    // );
    // let final_bys = vec![vec![0xb6, 0xf9, 0xde, 0x95], abi_bys.to_vec()].concat();
    // return (amount, router, Bytes::from(final_bys));
}


// write a test
#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;

    #[test]
    fn test_find_best_path_1inch() {
        let path = find_best_path_1nch(
            B160::from_str("0xf3ae5d769e153ef72b4e3591ac004e89f48107a1").unwrap(),
            B160::from_str("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap(),
            U256::from_str("100000000000000000000000000000000000000").unwrap(),
            "UNISWAP_V2".to_string()
        );
        println!("{:?}", path);
    }

    #[test]
    fn test_swap_1inch() {
        let (value, target, bys) = get_1inch_swap_args(
            B160::from_str("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap(),
            B160::from_str("0xf3ae5d769e153ef72b4e3591ac004e89f48107a1").unwrap(),
            U256::from_str("1257979238016341134939").unwrap(),
            B160::from_str("0xe8a7dB54F27FC7B855AE9BC950341878952EfF98").unwrap(),
            "ETH"
        );
        println!("{:?}", (value, target, hex::encode(bys)));
    }


    #[test]
    fn test_buy_token() {
        let (amount, router, data) = buy_token(
            B160::from_str("0xf3ae5d769e153ef72b4e3591ac004e89f48107a1").unwrap(),
            U256::from_str("1257979238016341134939").unwrap(),
            B160::from_str("0xe8a7dB54F27FC7B855AE9BC950341878952EfF98").unwrap(),
            "ETH"
        );
        println!("{:?}", (amount, router, data));
    }
}