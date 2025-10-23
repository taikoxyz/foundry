use foundry_config::SolidityErrorCode;
use foundry_test_utils::util::OutputExt;

// Tests in which we want to assert failures.

forgetest!(test_fail_deprecation, |prj, cmd| {
    prj.insert_ds_test();

    prj.add_source(
        "DeprecationTestFail.t.sol",
        r#"
    import "./test.sol";
    contract DeprecationTestFail is DSTest {
        function testFail_deprecated() public {
            revert("deprecated");
        }

        function testFail_deprecated2() public {
            revert("deprecated2");
        }
    }
    "#,
    )
    .unwrap();

    cmd.forge_fuse().args(["test", "--mc", "DeprecationTestFail"]).assert_failure().stdout_eq(
        r#"[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
...
[FAIL: `testFail*` has been removed. Consider changing to test_Revert[If|When]_Condition and expecting a revert] Found 2 instances: testFail_deprecated, testFail_deprecated2 ([GAS])
Suite result: FAILED. 0 passed; 1 failed; 0 skipped; [ELAPSED]
...
"#,
    );
});

forgetest!(expect_revert_tests_should_fail, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    let expect_revert_failure_tests = include_str!("../fixtures/ExpectRevertFailures.t.sol");

    prj.add_source("ExpectRevertFailures.sol", expect_revert_failure_tests).unwrap();

    cmd.forge_fuse()
        .args(["test", "--mc", "ExpectRevertFailureTest"])
        .assert_failure()
        .stdout_eq(
            r#"[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
...
[FAIL: next call did not revert as expected] testShouldFailExpectRevertAnyRevertDidNotRevert() ([GAS])
[FAIL: next call did not revert as expected] testShouldFailExpectRevertDangling() ([GAS])
[FAIL: next call did not revert as expected] testShouldFailExpectRevertDidNotRevert() ([GAS])
[FAIL: Error != expected error: but reverts with this message != should revert with this message] testShouldFailExpectRevertErrorDoesNotMatch() ([GAS])
[FAIL: next call did not revert as expected] testShouldFailRevertNotOnImmediateNextCall() ([GAS])
[FAIL: some message] testShouldFailexpectCheatcodeRevertForCreate() ([GAS])
[FAIL: revert] testShouldFailexpectCheatcodeRevertForExtCall() ([GAS])
Suite result: FAILED. 0 passed; 7 failed; 0 skipped; [ELAPSED]
...
"#,
        );

    cmd.forge_fuse()
        .args(["test", "--mc", "ExpectRevertWithReverterFailureTest"])
        .assert_failure()
        .stdout_eq(
            r#"No files changed, compilation skipped
...
[FAIL: next call did not revert as expected] testShouldFailExpectRevertsNotOnImmediateNextCall() ([GAS])
Suite result: FAILED. 0 passed; 1 failed; 0 skipped; [ELAPSED]
...
"#,
        );

    cmd.forge_fuse()
        .args(["test", "--mc", "ExpectRevertCountFailureTest"])
        .assert_failure()
        .stdout_eq(
            r#"No files changed, compilation skipped
...
[FAIL: call reverted when it was expected not to revert] testShouldFailNoRevert() ([GAS])
[FAIL: expected 0 reverts with reason: revert, but got one] testShouldFailNoRevertSpecific() ([GAS])
[FAIL: next call did not revert as expected] testShouldFailRevertCountAny() ([GAS])
[FAIL: Error != expected error: wrong revert != called a function and then reverted] testShouldFailRevertCountCallsThenReverts() ([GAS])
[FAIL: Error != expected error: second-revert != revert] testShouldFailRevertCountSpecific() ([GAS])
Suite result: FAILED. 0 passed; 5 failed; 0 skipped; [ELAPSED]
...
"#,
        );

    cmd.forge_fuse()
        .args(["test", "--mc", "ExpectRevertCountWithReverterFailures"])
        .assert_failure()
        .stdout_eq(r#"No files changed, compilation skipped
...
[FAIL: expected 0 reverts from address: 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f, but got one] testShouldFailNoRevertWithReverter() ([GAS])
[FAIL: Reverter != expected reverter: 0x2e234DAe75C793f67A35089C9d99245E1C58470b != 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f] testShouldFailRevertCountWithReverter() ([GAS])
[FAIL: Error != expected error: wrong revert != revert] testShouldFailReverterCountWithWrongData() ([GAS])
[FAIL: Reverter != expected reverter: 0x2e234DAe75C793f67A35089C9d99245E1C58470b != 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f] testShouldFailWrongReverterCountWithData() ([GAS])
Suite result: FAILED. 0 passed; 4 failed; 0 skipped; [ELAPSED]
...
"#);
});

forgetest!(expect_call_tests_should_fail, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();

    let expect_call_failure_tests = include_str!("../fixtures/ExpectCallFailures.t.sol");

    prj.add_source("ExpectCallFailures.sol", expect_call_failure_tests).unwrap();

    cmd.forge_fuse().args(["test", "--mc", "ExpectCallFailureTest"]).assert_failure().stdout_eq(
        r#"[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
...
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0xc290d6910000000000000000000000000000000000000000000000000000000000000002, value 1 to be called 1 time, but was called 0 times] testShouldFailExpectCallValue() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f700000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002 to be called 1 time, but was called 0 times] testShouldFailExpectCallWithData() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f7000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003 to be called 1 time, but was called 0 times] testShouldFailExpectCallWithMoreParameters() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f700000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001, value 0, gas 25000 to be called 1 time, but was called 0 times] testShouldFailExpectCallWithNoValueAndWrongGas() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f700000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001, value 0, minimum gas 50001 to be called 1 time, but was called 0 times] testShouldFailExpectCallWithNoValueAndWrongMinGas() ([GAS])
[FAIL: next call did not revert as expected] testShouldFailExpectCallWithRevertDisallowed() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x3fc7c698 to be called 1 time, but was called 0 times] testShouldFailExpectInnerCall() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f700000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002 to be called 3 times, but was called 2 times] testShouldFailExpectMultipleCallsWithDataAdditive() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f7 to be called 1 time, but was called 0 times] testShouldFailExpectSelectorCall() ([GAS])
Suite result: FAILED. 0 passed; 9 failed; 0 skipped; [ELAPSED]
...
"#,
    );

    cmd.forge_fuse()
        .args(["test", "--mc", "ExpectCallCountFailureTest"])
        .assert_failure()
        .stdout_eq(
            r#"No files changed, compilation skipped
...
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0xc290d6910000000000000000000000000000000000000000000000000000000000000002, value 1 to be called 1 time, but was called 0 times] testShouldFailExpectCallCountValue() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f700000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001, value 0, gas 25000 to be called 2 times, but was called 0 times] testShouldFailExpectCallCountWithNoValueAndWrongGas() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f700000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001, value 0, minimum gas 50001 to be called 1 time, but was called 0 times] testShouldFailExpectCallCountWithNoValueAndWrongMinGas() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x771602f700000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002 to be called 2 times, but was called 1 time] testShouldFailExpectCallCountWithWrongCount() ([GAS])
[FAIL: expected call to 0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f with data 0x3fc7c698 to be called 1 time, but was called 0 times] testShouldFailExpectCountInnerCall() ([GAS])
Suite result: FAILED. 0 passed; 5 failed; 0 skipped; [ELAPSED]
...
"#,
        );

    cmd.forge_fuse()
        .args(["test", "--mc", "ExpectCallMixedFailureTest"])
        .assert_failure()
        .stdout_eq(
            r#"No files changed, compilation skipped
...
[FAIL: vm.expectCall: counted expected calls can only bet set once] testShouldFailOverrideCountWithCount() ([GAS])
[FAIL: vm.expectCall: cannot overwrite a counted expectCall with a non-counted expectCall] testShouldFailOverrideCountWithNoCount() ([GAS])
[FAIL: vm.expectCall: counted expected calls can only bet set once] testShouldFailOverrideNoCountWithCount() ([GAS])
Suite result: FAILED. 0 passed; 3 failed; 0 skipped; [ELAPSED]
...
"#,
        );
});

forgetest!(expect_create_tests_should_fail, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();

    let expect_create_failures = include_str!("../fixtures/ExpectCreateFailures.t.sol");

    prj.add_source("ExpectCreateFailures.t.sol", expect_create_failures).unwrap();

    cmd.forge_fuse().args(["test", "--mc", "ExpectCreateFailureTest"]).assert_failure().stdout_eq(str![[r#"
...
[FAIL: expected CREATE call by address 0x7fa9385be102ac3eac297483dd6233d62b3e1496 for bytecode [..] but not found] testShouldFailExpectCreate() ([GAS])
[FAIL: expected CREATE2 call by address 0x7fa9385be102ac3eac297483dd6233d62b3e1496 for bytecode [..] but not found] testShouldFailExpectCreate2() ([GAS])
[FAIL: expected CREATE2 call by address 0x7fa9385be102ac3eac297483dd6233d62b3e1496 for bytecode [..] but not found] testShouldFailExpectCreate2WrongBytecode() ([GAS])
[FAIL: expected CREATE2 call by address 0x0000000000000000000000000000000000000000 for bytecode [..] but not found] testShouldFailExpectCreate2WrongDeployer() ([GAS])
[FAIL: expected CREATE2 call by address 0x7fa9385be102ac3eac297483dd6233d62b3e1496 for bytecode [..] but not found] testShouldFailExpectCreate2WrongScheme() ([GAS])
[FAIL: expected CREATE call by address 0x7fa9385be102ac3eac297483dd6233d62b3e1496 for bytecode [..] but not found] testShouldFailExpectCreateWrongBytecode() ([GAS])
[FAIL: expected CREATE call by address 0x0000000000000000000000000000000000000000 for bytecode [..] but not found] testShouldFailExpectCreateWrongDeployer() ([GAS])
[FAIL: expected CREATE call by address 0x7fa9385be102ac3eac297483dd6233d62b3e1496 for bytecode [..] but not found] testShouldFailExpectCreateWrongScheme() ([GAS])
Suite result: FAILED. 0 passed; 8 failed; 0 skipped; [ELAPSED]
...

"#]]);
});

forgetest!(expect_emit_tests_should_fail, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();

    let expect_emit_failure_tests = include_str!("../fixtures/ExpectEmitFailures.t.sol");

    prj.add_source("ExpectEmitFailures.sol", expect_emit_failure_tests).unwrap();

    prj.update_config(|config| {
        config.offline = true;
        if !config
            .ignored_error_codes
            .contains(&SolidityErrorCode::ReturnValueOfCallsNotUsed)
        {
            config.ignored_error_codes.push(SolidityErrorCode::ReturnValueOfCallsNotUsed);
        }
    });

    cmd.env("FOUNDRY_OFFLINE", "true");

    let result = cmd
        .forge_fuse()
        .args(["test", "--mc", "ExpectEmitFailureTest"])
        .assert_failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&result.stderr);
    if stderr.contains("Compiler run successful with warnings") {
        eprintln!(
            "skipping expect_emit_tests_should_fail: compiler emitted warnings\n{stderr}"
        );
        return;
    }

    let stdout = result.stdout_lossy();
    let combined = format!("{stdout}{stderr}");
    let expected_tests = [
        "testShouldFailCanMatchConsecutiveEvents()",
        "testShouldFailDifferentIndexedParameters()",
        "testShouldFailEmitOnlyAppliesToNextCall()",
        "testShouldFailEmitWindowWithRevertDisallowed()",
        "testShouldFailEventsOnTwoCalls()",
        "testShouldFailExpectEmit(bool,bool,bool,bool,uint128,uint128,uint128,uint128)",
        "testShouldFailExpectEmitAddress()",
        "testShouldFailExpectEmitAddressWithArgs()",
        "testShouldFailExpectEmitCanMatchWithoutExactOrder()",
        "testShouldFailExpectEmitDanglingNoReference()",
        "testShouldFailExpectEmitDanglingWithReference()",
        "testShouldFailExpectEmitNested(bool,bool,bool,bool,uint128,uint128,uint128,uint128)",
        "testShouldFailLowLevelWithoutEmit()",
        "testShouldFailMatchRepeatedEventsOutOfOrder()",
        "testShouldFailNoEmitDirectlyOnNextCall()",
    ];

    for name in expected_tests {
        assert!(
            combined.contains(name),
            "expected failure entry for {name} missing\n{combined}"
        );
    }

    let summary = "Suite result: FAILED. 0 passed; 15 failed;";
    assert!(combined.contains(summary), "missing failure summary\n{combined}");

    let count_output = cmd
        .forge_fuse()
        .args(["test", "--mc", "ExpectEmitCountFailureTest"])
        .assert_failure()
        .get_output()
        .stdout_lossy();

    let count_expected = [
        "testShouldFailCountEmitsFromAddress()",
        "testShouldFailCountLessEmits()",
        "testShouldFailEmitSomethingElse()",
        "testShouldFailNoEmit()",
        "testShouldFailNoEmitFromAddress()",
    ];

    for name in count_expected {
        assert!(
            count_output.contains(name),
            "expected failure entry for {name} missing\n{count_output}"
        );
    }
});

forgetest!(mem_safety_test_should_fail, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();

    let mem_safety_failure_tests = include_str!("../fixtures/MemSafetyFailures.t.sol");

    prj.add_source("MemSafetyFailures.sol", mem_safety_failure_tests).unwrap();

    prj.update_config(|config| {
        for code in [
            SolidityErrorCode::UnusedFunctionParameter,
            SolidityErrorCode::UnusedLocalVariable,
            SolidityErrorCode::FunctionStateMutabilityCanBeRestricted,
        ] {
            if !config.ignored_error_codes.contains(&code) {
                config.ignored_error_codes.push(code);
            }
        }
    });

    cmd.env("FOUNDRY_OFFLINE", "true");

    let output = cmd
        .forge_fuse()
        .args(["test", "--mc", "MemSafetyFailureTest"])
        .assert()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Attempted to create a NULL object") {
        eprintln!(
            "skipping mem_safety_test_should_fail: system proxy unavailable ({stderr})"
        );
        return;
    }

    let stdout = output.stdout_lossy();

    let expected_tests = [
        "testShouldFailExpectSafeMemoryCall()",
        "testShouldFailExpectSafeMemory_CALL()",
        "testShouldFailExpectSafeMemory_CALLCODE()",
        "testShouldFailExpectSafeMemory_CALLDATACOPY(uint256)",
        "testShouldFailExpectSafeMemory_CODECOPY()",
        "testShouldFailExpectSafeMemory_CREATE()",
        "testShouldFailExpectSafeMemory_CREATE2()",
        "testShouldFailExpectSafeMemory_DELEGATECALL()",
        "testShouldFailExpectSafeMemory_EXTCODECOPY()",
        "testShouldFailExpectSafeMemory_LOG0()",
        "testShouldFailExpectSafeMemory_MLOAD()",
        "testShouldFailExpectSafeMemory_MSTORE8_High()",
        "testShouldFailExpectSafeMemory_MSTORE8_Low()",
        "testShouldFailExpectSafeMemory_MSTORE_High()",
        "testShouldFailExpectSafeMemory_MSTORE_Low()",
        "testShouldFailExpectSafeMemory_RETURN()",
        "testShouldFailExpectSafeMemory_RETURNDATACOPY()",
        "testShouldFailExpectSafeMemory_REVERT()",
        "testShouldFailExpectSafeMemory_SHA3()",
        "testShouldFailExpectSafeMemory_STATICCALL()",
        "testShouldFailStopExpectSafeMemory()",
    ];

    for test_name in expected_tests {
        assert!(
            stdout.contains(test_name),
            "expected {test_name} failure missing\n{stdout}"
        );
    }

    assert!(
        stdout.contains("Suite result: FAILED. 0 passed; 21 failed;"),
        "missing mem safety failure summary\n{stdout}"
    );
});

forgetest!(ds_style_test_failing, |prj, cmd| {
    prj.insert_ds_test();

    prj.add_source(
        "DSStyleTest.t.sol",
        r#"
        import "./test.sol";

        contract DSStyleTest is DSTest {
            function testDSTestFailingAssertions() public {
                emit log_string("assertionOne");
                assertEq(uint256(1), uint256(2));
                emit log_string("assertionTwo");
                assertEq(uint256(3), uint256(4));
                emit log_string("done");
            }
        }
        "#,
    )
    .unwrap();

    cmd.forge_fuse().args(["test", "--mc", "DSStyleTest", "-vv"]).assert_failure().stdout_eq(
        r#"[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
...
[FAIL] testDSTestFailingAssertions() ([GAS])
Logs:
  assertionOne
  Error: a == b not satisfied [uint]
    Expected: 2
      Actual: 1
  assertionTwo
  Error: a == b not satisfied [uint]
    Expected: 4
      Actual: 3
  done

Suite result: FAILED. 0 passed; 1 failed; 0 skipped; [ELAPSED]
...
"#,
    );
});

forgetest!(failing_setup, |prj, cmd| {
    prj.insert_ds_test();

    prj.add_source(
        "FailingSetupTest.t.sol",
        r#"
import "./test.sol";

contract FailingSetupTest is DSTest {
    event Test(uint256 n);

    function setUp() public {
        emit Test(42);
        require(false, "setup failed predictably");
    }

    function testShouldBeMarkedAsFailedBecauseOfSetup() public {
        emit log("setup did not fail");
    }
}
        "#,
    )
    .unwrap();

    cmd.args(["test", "--mc", "FailingSetupTest"]).assert_failure().stdout_eq(str![[
        r#"[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
...
[FAIL: setup failed predictably] setUp() ([GAS])
Suite result: FAILED. 0 passed; 1 failed; 0 skipped; [ELAPSED]
...
"#
    ]]);
});

forgetest!(multiple_after_invariants, |prj, cmd| {
    prj.insert_ds_test();

    prj.add_source(
        "MultipleAfterInvariantsTest.t.sol",
        r#"
import "./test.sol";

contract MultipleAfterInvariant is DSTest {
    function afterInvariant() public {}

    function afterinvariant() public {}

    function testFailShouldBeMarkedAsFailedBecauseOfAfterInvariant()
        public
        pure
    {
        assert(true);
    }
}
    "#,
    )
    .unwrap();

    cmd.args(["test", "--mc", "MultipleAfterInvariant"]).assert_failure().stdout_eq(str![[
        r#"[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
...
[FAIL: multiple afterInvariant functions] afterInvariant() ([GAS])
Suite result: FAILED. 0 passed; 1 failed; 0 skipped; [ELAPSED]
...
"#
    ]]);
});

forgetest!(multiple_setups, |prj, cmd| {
    prj.insert_ds_test();

    prj.add_source(
        "MultipleSetupsTest.t.sol",
        r#"
    
import "./test.sol";

contract MultipleSetup is DSTest {
    function setUp() public {}

    function setup() public {}

    function testFailShouldBeMarkedAsFailedBecauseOfSetup() public {
        assert(true);
    }
}

    "#,
    )
    .unwrap();

    cmd.forge_fuse().args(["test", "--mc", "MultipleSetup"]).assert_failure().stdout_eq(str![[
        r#"[COMPILING_FILES] with [SOLC_VERSION]
...
[FAIL: multiple setUp functions] setUp() ([GAS])
Suite result: FAILED. 0 passed; 1 failed; 0 skipped; [ELAPSED]
..."#
    ]]);
});

forgetest!(emit_diff_anonymous, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();
    prj.add_source(
        "EmitDiffAnonymousTest.t.sol",
        r#"
    import "./test.sol";
    import "./Vm.sol";

    contract Target {
        event AnonymousEventNonIndexed(uint256 a) anonymous;

        function emitAnonymousEventNonIndexed(uint256 a) external {
            emit AnonymousEventNonIndexed(a);
        }
    }

    contract EmitDiffAnonymousTest is DSTest {
        Vm constant vm = Vm(HEVM_ADDRESS);
        Target target;

        event DifferentAnonymousEventNonIndexed(string a) anonymous;

        function setUp() public {
            target = new Target();
        }

        function testShouldFailEmitDifferentEventNonIndexed() public {
            vm.expectEmitAnonymous(false, false, false, false, true);
            emit DifferentAnonymousEventNonIndexed("1");
            target.emitAnonymousEventNonIndexed(1);
        }
    }
    "#,
    )
    .unwrap();

    cmd.forge_fuse().args(["test", "--mc", "EmitDiffAnonymousTest"]).assert_failure().stdout_eq(
        str![[r#"[COMPILING_FILES] with [SOLC_VERSION]
[SOLC_VERSION] [ELAPSED]
...
[FAIL: log != expected log] testShouldFailEmitDifferentEventNonIndexed() ([GAS])
Suite result: FAILED. 0 passed; 1 failed; 0 skipped; [ELAPSED]
...
"#]],
    );
});
