// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "../libs/context.sol";

interface ERC20 {
    function totalSupply() external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function mint(address _addr, uint256 value) external;
    function balanceOf(address account) external view returns (uint256);
}

contract infinite_mint {
    // target contract
    ERC20 public target;
    // total supply of the target contract
    uint256 public current_total_supply;

    // context object that interacts with the API
    Context public ctx = Context(0x8891e33ba3c6A7b4E020A6180Eb07f4AED2d70CE);

    // call with bytes defined in the config file
    constructor(){
        target = ERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
        current_total_supply = target.totalSupply();
    }

    // invariant: mint should always reverts
    function invariant_infinite_mint0() public returns (bool) {
        (address[] memory contracts, bytes[] memory data) = ctx.get_affected_contracts();
        // loop over all called contracts during the transaction
        for (uint i = 0; i < contracts.length; i++) {
            // check if the call target is the target contract
            if (contracts[i] == address(target)) {
                // decode the data
                if (data[i].length < 32 * 3) {
                    continue;
                }
                (bytes4 sig, address _addr, uint256 value) = abi.decode(data[i], (bytes4, address, uint256));
                // check if the call is mint and the value is greater than 0
                if (sig == bytes4(keccak256("mint(address,uint256)")) && value > 0) {
                    return true;
                }
            }
        }
        return false;
    }


    // invariant: total supply should not change and sum of balance of affected accounts should not change
    function invariant_infinite_mint1() public returns (bool) {
        // check if the total supply changed
        if (target.totalSupply() != current_total_supply) {
            ctx.print_string("total supply changed");
            return true;
        }
        // get affected accounts
        (address[] memory accounts, address[] memory contracts) = ctx.get_affected_accounts_ierc20();
        int256 diff = 0;
        // loop over all affected accounts used in IERC20 calls
        for (uint i = 0; i < accounts.length; i++) {
            // check if the call target is the target contract
            if (contracts[i] == address(target)) {
                // get the balance of the account before the transaction
                uint256 prev_balance = uint256(bytes32(
                    ctx.call_prev_state(
                        contracts[i],
                        address(this),
                        abi.encodeWithSignature("balanceOf(address)", accounts[i]), 0
                    )
                ));
                // get the balance of the account after the transaction
                uint256 curr_balance = uint256(ERC20(contracts[i]).balanceOf(accounts[i]));
                // calculate the difference
                diff -= int256(prev_balance);
                diff += int256(curr_balance);
            }
        }
        // the difference should be 0
        if (diff != 0) {
            ctx.print_string("diff != 0");
            return true;
        }
        return false;
    }

    function test_1() public {
        ctx.buy_token(address(target), 1e18); // buy 1 ETH worth of token
        ctx.print_int("balanceOf(target)", target.balanceOf(address(this))); // around 1800e6 USDC

        // create a test call
        ctx.test_call(address(target), address(this), abi.encodeWithSignature("transfer(address,uint256)", address(0x1),1e6), 0);

        // not token swapped during last call
        require(ctx.contains_swap() == false);
        require(ctx.get_affected_pairs().length == 0);

        // check invariants
        require(invariant_infinite_mint0() == false);
        require(invariant_infinite_mint1() == false);

        ctx.print_string("all good");
    }
}
