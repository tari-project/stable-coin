// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::fmt;
use tari_crypto::tari_utilities::byte_array::ByteArray;
use tari_engine_types::commit_result::{FinalizeResult, RejectReason, TransactionResult};
use tari_engine_types::instruction_result::InstructionResult;
use tari_engine_types::substate::{SubstateDiff, SubstateId, SubstateValue};
use tari_template_abi::Type;
use tari_template_lib::models::{Amount, BucketId, NonFungibleId};
use tari_wallet_daemon_client::types::TransactionWaitResultResponse;

pub fn print_result(resp: &TransactionWaitResultResponse) {
    println!("Transaction ID: {}", resp.transaction_id);

    if let Some(ref result) = resp.result {
        summarize_finalize_result(result);
    }

    println!();
    println!("Fee: {}", resp.final_fee);
    // println!("Time taken: {:?}", time_taken);
    println!();
    if let Some(ref result) = resp.result {
        println!("OVERALL DECISION: {}", result.result);
    } else {
        println!("STATUS: {:?}", resp.status);
    }
}

pub fn print_substate_diff(diff: &SubstateDiff) {
    for (address, substate) in diff.up_iter() {
        println!("️🌲 UP substate {} (v{})", address, substate.version(),);
        match substate.substate_value() {
            SubstateValue::Component(component) => {
                println!("      ▶ component ({}): {}", component.module_name, address,);
            }
            SubstateValue::Resource(_) => {
                println!("      ▶ resource: {}", address);
            }
            SubstateValue::TransactionReceipt(_) => {
                println!("      ▶ transaction_receipt: {}", address);
            }
            SubstateValue::Vault(vault) => {
                println!("      ▶ vault: {} {}", address, vault.resource_address());
            }
            SubstateValue::NonFungible(_) => {
                println!("      ▶ NFT: {}", address);
            }
            SubstateValue::UnclaimedConfidentialOutput(_) => {
                println!("      ▶ Layer 1 commitment: {}", address);
            }
            SubstateValue::NonFungibleIndex(index) => {
                let referenced_address = SubstateId::from(index.referenced_address().clone());
                println!(
                    "      ▶ NFT index {} referencing {}",
                    address, referenced_address
                );
            }
            SubstateValue::FeeClaim(fee_claim) => {
                println!("      ▶ Fee claim: {}", address);
                println!("        ▶ Amount: {}", fee_claim.amount);
                println!(
                    "        ▶ validator: {}",
                    hex::encode(fee_claim.validator_public_key.as_bytes())
                );
            }
        }
        println!();
    }
    for (address, version) in diff.down_iter() {
        println!("🗑️ DOWN substate {} v{}", address, version,);
        println!();
    }
}

pub fn summarize_finalize_result(finalize: &FinalizeResult) {
    println!("========= Substates =========");
    match finalize.result {
        TransactionResult::Accept(ref diff) => print_substate_diff(diff),
        TransactionResult::AcceptFeeRejectRest(ref diff, ref reason) => {
            print_substate_diff(diff);
            print_reject_reason(reason);
        }
        TransactionResult::Reject(ref reason) => print_reject_reason(reason),
    }

    println!("========= Return Values =========");
    print_execution_results(&finalize.execution_results);

    println!();
    println!("========= LOGS =========");
    for log in &finalize.logs {
        println!("{}", log);
    }
}

fn print_reject_reason(reason: &RejectReason) {
    println!("❌️ Transaction rejected: {}", reason);
}

fn display_vec<W: fmt::Write>(
    writer: &mut W,
    ty: &Type,
    result: &InstructionResult,
) -> fmt::Result {
    fn stringify_slice<T: fmt::Display>(slice: &[T]) -> String {
        slice
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    match &ty {
        Type::Unit => {}
        Type::Bool => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<bool>>().unwrap())
            )?;
        }
        Type::I8 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<i8>>().unwrap())
            )?;
        }
        Type::I16 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<i16>>().unwrap())
            )?;
        }
        Type::I32 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<i32>>().unwrap())
            )?;
        }
        Type::I64 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<i64>>().unwrap())
            )?;
        }
        Type::I128 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<i128>>().unwrap())
            )?;
        }
        Type::U8 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<u8>>().unwrap())
            )?;
        }
        Type::U16 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<u16>>().unwrap())
            )?;
        }
        Type::U32 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<u32>>().unwrap())
            )?;
        }
        Type::U64 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<u64>>().unwrap())
            )?;
        }
        Type::U128 => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<u128>>().unwrap())
            )?;
        }
        Type::String => {
            write!(
                writer,
                "{}",
                result.decode::<Vec<String>>().unwrap().join(", ")
            )?;
        }
        Type::Vec(ty) => {
            let mut vec_ty = String::new();
            display_vec(&mut vec_ty, ty, result)?;
            match &**ty {
                Type::Other { name } => {
                    write!(writer, "Vec<{}>: {}", name, vec_ty)?;
                }
                _ => {
                    write!(writer, "Vec<{:?}>: {}", ty, vec_ty)?;
                }
            }
        }
        Type::Tuple(subtypes) => {
            let str = format_tuple(subtypes, result);
            write!(writer, "{}", str)?;
        }
        Type::Other { name } if name == "Amount" => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<Amount>>().unwrap())
            )?;
        }
        Type::Other { name } if name == "NonFungibleId" => {
            write!(
                writer,
                "{}",
                stringify_slice(&result.decode::<Vec<NonFungibleId>>().unwrap())
            )?;
        }
        Type::Other { .. } => {
            write!(
                writer,
                "{}",
                serde_json::to_string_pretty(&result.indexed).unwrap()
            )?;
        }
    }
    Ok(())
}

fn format_tuple(subtypes: &[Type], result: &InstructionResult) -> String {
    let tuple_type = Type::Tuple(subtypes.to_vec());
    let result_json = serde_json::to_string(&result.indexed).unwrap();
    format!("{}: {}", tuple_type, result_json)
}

pub fn print_execution_results(results: &[InstructionResult]) {
    for result in results {
        match &result.return_type {
            Type::Unit => {}
            Type::Bool => {
                println!("bool: {}", result.decode::<bool>().unwrap());
            }
            Type::I8 => {
                println!("i8: {}", result.decode::<i8>().unwrap());
            }
            Type::I16 => {
                println!("i16: {}", result.decode::<i16>().unwrap());
            }
            Type::I32 => {
                println!("i32: {}", result.decode::<i32>().unwrap());
            }
            Type::I64 => {
                println!("i64: {}", result.decode::<i64>().unwrap());
            }
            Type::I128 => {
                println!("i128: {}", result.decode::<i128>().unwrap());
            }
            Type::U8 => {
                println!("u8: {}", result.decode::<u8>().unwrap());
            }
            Type::U16 => {
                println!("u16: {}", result.decode::<u16>().unwrap());
            }
            Type::U32 => {
                println!("u32: {}", result.decode::<u32>().unwrap());
            }
            Type::U64 => {
                println!("u64: {}", result.decode::<u64>().unwrap());
            }
            Type::U128 => {
                println!("u128: {}", result.decode::<u128>().unwrap());
            }
            Type::String => {
                println!("string: {}", result.decode::<String>().unwrap());
            }
            Type::Vec(ty) => {
                let mut vec_ty = String::new();
                display_vec(&mut vec_ty, ty, result).unwrap();
                match &**ty {
                    Type::Other { name } => {
                        println!("Vec<{}>: {}", name, vec_ty);
                    }
                    _ => {
                        println!("Vec<{:?}>: {}", ty, vec_ty);
                    }
                }
            }
            Type::Tuple(subtypes) => {
                let str = format_tuple(subtypes, result);
                println!("{}", str);
            }
            Type::Other { ref name } if name == "Amount" => {
                println!("{}: {}", name, result.decode::<Amount>().unwrap());
            }
            Type::Other { ref name } if name == "Bucket" => {
                println!("{}: {}", name, result.decode::<BucketId>().unwrap());
            }
            Type::Other { ref name } => {
                println!(
                    "{}: {}",
                    name,
                    serde_json::to_string_pretty(&result.indexed).unwrap()
                );
            }
        }
    }
}
