// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

interface Context {
    function get_caller() external view returns (address);
    function get_target() external view returns (address);
    function get_value() external view returns (uint256);
    function get_data() external view returns (bytes memory);


    function get_affected_contracts() external view returns (address[] memory, bytes[] memory);
    function get_affected_accounts_ierc20() external view returns (address[] memory, address[] memory);

    function call_prev_state(address _contract, address caller, bytes memory data, uint256 value) external view returns (bytes memory);

    // Test only
    function test_call(address _contract, address caller, bytes memory data, uint256 value) external returns (bytes memory);
}
