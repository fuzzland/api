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
// get last transaction details
function get_caller() external view returns (address);
function get_target() external view returns (address);
function get_value() external view returns (uint256);
function get_data() external view returns (bytes memory);

// get all called contracts during the last transaction
function get_affected_contracts() external view returns (address[] memory, bytes[] memory);
// get all affected accounts due to ERC20 transfer during the last transaction
function get_affected_accounts_ierc20() external view returns (address[] memory, address[] memory);

// call a function with the state before the last transaction
function call_prev_state(address _contract, address caller, bytes memory data, uint256 value) external view returns (bytes memory);

// more functions can be added per demands
```

An example is provided in the `example` folder, which detects the infinite mints for contract `0x6AB5F1f81008c3F4481F7EF5c3304AD183DAd236` on BSC.
You can compile using following command:
```bash
cd example
solc infinite_mint.sol --base-path . --include-path .. --abi --bin --overwrite -o ./out
```

A test kit is also provided (you need to setup Rust using [rustup](https://rustup.rs/)):
```bash
# build testkit
cargo build
# run the testkit
./target/debug/api-cli "./example/out/infinite_mint*" BSC
```
which calls all the `test_*` functions in the `infinite_mint.sol` contract. 
Unknown storage slots and unknown contracts are automatically fetched from BSC.