// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

interface Context {
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
}
