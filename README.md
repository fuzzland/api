# FuzzLand Spec (Solidity)
The FuzzLand API specification for Solidity Spec developments outlines the requirements for using FuzzLand to test Solidity smart contracts for violations of invariants. To use FuzzLand, the user must provide their Solidity smart contracts which specify the conditions under which a state violates invariants. These specifications should be in the following format:

```solidity
contract Invariant {

    // invariants functions
    function invariant_1() public returns (bool) {
        // do something
        return true;
    }

    function invariant_2() public returns (bool) {
        // do something
        return true;
    }
    ...
    
    // test functions
    function test_1() public {
    // do something
    }
    
    function test_2() public {
    // do something
    }
    
    ...
}
```

In the FuzzLand API specification for Solidity Spec developments, the invariant functions play a crucial role in identifying bugs in the Solidity smart contracts. If the output of an invariant function is true, it indicates that the state violates the specified invariant, revealing the presence of a bug. Conversely, if the output is false, it signifies that the invariant is not violated, indicating the absence of any bugs.

Test functions are not mandatory for deployment, but they can be useful for testing the invariant functions. These test functions allow the user to check the accuracy of the invariant functions, ensuring that they are working as intended.

The invariant functions are called after the transaction is executed. This sequence allows the invariant functions to inspect the state of the smart contract before and after the transaction is completed, enabling them to identify any violations of the specified invariants.

There is a Context contract deployed at `0x8891e33ba3c6A7b4E020A6180Eb07f4AED2d70CE` allowing the invariant contracts to interact with the FuzzLand APIs. The contract provides the following functions:

```solidity
// get the call information
function get_caller() external view returns (address);
function get_target() external view returns (address);
function get_value() external view returns (uint256);
function get_data() external view returns (bytes memory);

// get the all contracts that have been called during the transaction
function get_affected_contracts() external view returns (address[] calldata, bytes[] calldata);
// get the all accounts that have been used in IERC20 calls during the transaction (e.g., caller and receiver of transfer)
function get_affected_accounts_ierc20() external view returns (address[] calldata, address[] calldata);

// does last transaction contain a swap?
function contains_swap() external view returns (bool);
// get the all pairs that have been swapped during the transaction
function get_affected_pairs() external view returns (address[] calldata);

// call a contract on state before the transaction
function call_prev_state(address _contract, address caller, bytes memory data, uint256 value) external view returns (bytes memory);
// sell the token on best offer and receive ETH
function sell_token_to_eth_best_path(address token, uint256 amountTokenInWei) external;

// Test only
function test_call(address _contract, address caller, bytes memory data, uint256 value) external returns (bytes memory);

// set ETH balance of an account
function set_balance(address account, uint256 amount) external;
// buy token with ETH
function buy_token(address token, uint256 amountETHInWei) external;

// Print functions
function print_int(string memory key, uint256 i) external;
function print_address(string memory key, address a) external;
function print_string(string memory s) external;

// more functions can be added per demands
```

An example is provided in the `example` folder, which detects the infinite mints for contract `0x6AB5F1f81008c3F4481F7EF5c3304AD183DAd236` on BSC.
You can compile using following command:
```bash
cd example
solc infinite_mint.sol --base-path . --include-path .. --abi --bin --overwrite -o ./out
```

## Testkit
A test kit is also provided (you need to setup Rust using [rustup](https://rustup.rs/)):
```bash
# build testkit
cargo build
# run the testkit
./target/debug/api-cli "./example/out/infinite_mint*" ETH
```
which calls all the `test_*` functions in the `infinite_mint.sol` contract.
Unknown storage slots and unknown contracts are automatically fetched from BSC.

You can also use Docker to run the testkit, where both the testkit and the example contract have already been built inside the container:
```bash
docker run -it fuzzland/api-client bash
# inside the container
$ ./target/debug/api-cli "./example/out/infinite_mint*" ETH
```

To make testing faster, you can replace public RPC to your own Infura / QuickNode / etc RPC @ https://github.com/fuzzland/api/blob/main/src/cli.rs#L47-L52

