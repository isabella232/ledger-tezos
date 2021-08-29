/*******************************************************************************
*   (c) 2021 Zondax GmbH
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
use nom::{
    call, cond, do_parse,
    number::complete::{be_u32, le_u8},
    take, IResult,
};

use crate::{
    crypto::Curve,
    handlers::parser_common::ParserError,
    parser::{boolean, public_key_hash, Zarith},
};

use super::ContractID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entrypoint<'b> {
    Default,
    Root,
    Do,
    SetDelegate,
    RemoveDelegate,
    Custom(&'b [u8]),
}

impl<'b> Entrypoint<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        let (mut rem, tag) = le_u8(input)?;

        let data = match tag {
            0x00 => Self::Default,
            0x01 => Self::Root,
            0x02 => Self::Do,
            0x03 => Self::SetDelegate,
            0x04 => Self::RemoveDelegate,
            0xFF => {
                let (rem2, length) = le_u8(rem)?;
                let (rem2, name) = take!(rem2, length)?;
                rem = rem2;

                Self::Custom(name)
            }
            _ => return Err(ParserError::parser_invalid_contract_name.into()),
        };

        Ok((rem, data))
    }
}

impl<'b> core::fmt::Display for Entrypoint<'b> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Entrypoint::Default => write!(f, "default"),
            Entrypoint::Root => write!(f, "root"),
            Entrypoint::Do => write!(f, "do"),
            Entrypoint::SetDelegate => write!(f, "set_delegate"),
            Entrypoint::RemoveDelegate => write!(f, "remove_delegate"),
            Entrypoint::Custom(custom) => {
                let custom = core::str::from_utf8(custom).expect("custom entrypoint was not utf8");
                f.write_str(custom)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Parameters<'b> {
    entrypoint: Entrypoint<'b>,
    michelson: &'b [u8],
}

impl<'b> Parameters<'b> {
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        #[rustfmt::skip]
        let (rem, (entrypoint, michelson)) =
            do_parse!(input,
                entrypoint: call!(Entrypoint::from_bytes) >>
                length: be_u32 >>
                out: take!(length) >>
                (entrypoint, out)
            )?;

        Ok((
            rem,
            Self {
                entrypoint,
                michelson,
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, property::Property)]
#[property(mut(disable), get(public), set(disable))]
pub struct Transfer<'b> {
    source: (Curve, &'b [u8; 20]),
    fee: Zarith<'b>,
    counter: Zarith<'b>,
    gas_limit: Zarith<'b>,
    storage_limit: Zarith<'b>,
    amount: Zarith<'b>,
    destination: ContractID<'b>,
    parameters: Option<Parameters<'b>>,
}

impl<'b> Transfer<'b> {
    #[cfg(not(test))]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        #[rustfmt::skip]
        let (rem, (source, fee, counter, gas_limit, storage_limit, amount, destination, parameters)) =
            do_parse! {input,
                source: public_key_hash >>
                fee: call!(Zarith::from_bytes, false) >>
                counter: call!(Zarith::from_bytes, false) >>
                gas_limit: call!(Zarith::from_bytes, false) >>
                storage_limit: call!(Zarith::from_bytes, false) >>
                amount: call!(Zarith::from_bytes, false) >>
                destination: call!(ContractID::from_bytes) >>
                has_params: boolean >>
                params: cond!(has_params, Parameters::from_bytes) >>
                (source, fee, counter, gas_limit, storage_limit, amount, destination, params)
            }?;

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                amount,
                destination,
                parameters,
            },
        ))
    }

    #[cfg(test)]
    pub fn from_bytes(input: &'b [u8]) -> IResult<&[u8], Self, ParserError> {
        use nom::dbg_basic;
        use std::println;

        #[rustfmt::skip]
        let (rem, (source, fee, counter, gas_limit, storage_limit, amount, destination, parameters)) =
            dbg_basic! {input,
                do_parse!(
                source: public_key_hash >>
                fee: call!(Zarith::from_bytes, false) >>
                counter: call!(Zarith::from_bytes, false) >>
                gas_limit: call!(Zarith::from_bytes, false) >>
                storage_limit: call!(Zarith::from_bytes, false) >>
                amount: call!(Zarith::from_bytes, false) >>
                destination: call!(ContractID::from_bytes) >>
                has_params: boolean >>
                params: cond!(has_params, Parameters::from_bytes) >>
                (source, fee, counter, gas_limit, storage_limit, amount, destination, params)
            )}?;

        Ok((
            rem,
            Self {
                source,
                fee,
                counter,
                gas_limit,
                storage_limit,
                amount,
                destination,
                parameters,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{Entrypoint, Transfer};

    #[test]
    fn entrypoint() {
        let (_, default) =
            Entrypoint::from_bytes(&[0]).expect("failed to parse default entrypoint");
        assert_eq!(default, Entrypoint::Default);

        let (_, root) = Entrypoint::from_bytes(&[1]).expect("failed to parse root entrypoint");
        assert_eq!(root, Entrypoint::Root);

        let (_, do_entrypoint) =
            Entrypoint::from_bytes(&[2]).expect("failed to parse do entrypoint");
        assert_eq!(do_entrypoint, Entrypoint::Do);

        let (_, set_delegate) =
            Entrypoint::from_bytes(&[3]).expect("failed to parse set_delegate entrypoint");
        assert_eq!(set_delegate, Entrypoint::SetDelegate);

        let (_, remove_delegate) =
            Entrypoint::from_bytes(&[4]).expect("failed to parse remove_delegate entrypoint");
        assert_eq!(remove_delegate, Entrypoint::RemoveDelegate);

        let (rem, custom) = Entrypoint::from_bytes(&[0xFF, 0x03, 0x61, 0x62, 0x63, 0xaa])
            .expect("failed to parse custom entrypoint");
        assert_eq!(rem.len(), 1);
        assert_eq!(custom, Entrypoint::Custom(b"abc"));
    }

    #[test]
    #[should_panic(expected = "Incomplete(Size(8))")]
    fn entrypoint_eof() {
        Entrypoint::from_bytes(&[0xFF, 10, 0x61, 0x62]).expect("failed to parse custom entrypoint");
    }

    mod parameters {
        use super::{super::Parameters, Entrypoint};

        #[test]
        fn manual() {
            const MICHELSON_CODE: &[u8] = &[0xab, 0xcd];

            let mut input = std::vec![0];
            input.extend_from_slice(&(MICHELSON_CODE.len() as u32).to_be_bytes()[..]);
            input.extend_from_slice(MICHELSON_CODE);
            input.push(0xAA); //dummy byte to verify length

            let (rem, parameters) =
                Parameters::from_bytes(&input).expect("faled to parse parameters");

            assert_eq!(rem.len(), 1);
            assert_eq!(
                parameters,
                Parameters {
                    entrypoint: Entrypoint::Default,
                    michelson: &[0xab, 0xcd]
                }
            )
        }

        #[test]
        fn simple() {
            const INPUT_HEX: &str = "02000000070a000000020202";

            let input = hex::decode(INPUT_HEX).expect("invalid hex input");

            let (_, parameters) =
                Parameters::from_bytes(&input).expect("faled to parse parameters");

            assert_eq!(
                parameters,
                Parameters {
                    entrypoint: Entrypoint::Do,
                    michelson: &input[5..]
                }
            )
        }

        #[test]
        #[should_panic(expected = "Incomplete(Size(8))")]
        fn manual_eof() {
            const MICHELSON_CODE: &[u8] = &[0xab, 0xcd];

            let mut input = std::vec![0];
            input.extend_from_slice(&(10u32).to_be_bytes()[..]);
            input.extend_from_slice(MICHELSON_CODE);

            Parameters::from_bytes(&input).expect("failed to parse parameters");
        }
    }

    #[test]
    fn transfer() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 e807\
                                 000035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 00";

        let mut input = hex::decode(INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(&[0xDE, 0xEA, 0xBE, 0xEF]);

        let (rem, parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 4);

        let expected = Transfer {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
            fee: Zarith {
                is_negative: None,
                bytes: &input[21..23],
            },
            counter: Zarith {
                is_negative: None,
                bytes: &input[23..24],
            },
            gas_limit: Zarith {
                is_negative: None,
                bytes: &input[24..25],
            },
            storage_limit: Zarith {
                is_negative: None,
                bytes: &input[25..26],
            },
            amount: Zarith {
                is_negative: None,
                bytes: &input[26..28],
            },
            destination: ContractID::Implicit(
                Curve::Bip32Ed25519,
                arrayref::array_ref!(input, 28 + 2, 20),
            ),
            parameters: None,
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    #[should_panic(expected = "Incomplete")]
    fn transfer_eof() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd904e01";

        let mut input = hex::decode(INPUT_HEX).expect("invalid input hex");
        input.extend_from_slice(&[0xDE, 0xEA, 0xBE, 0xEF]);

        let (rem, _parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 4);
    }

    #[test]
    fn contract_call() {
        const INPUT_HEX: &str = "0035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 904e\
                                 01\
                                 0a\
                                 0a\
                                 e807\
                                 000035e993d8c7aaa42b5e3ccd86a33390ececc73abd\
                                 ff\
                                 02000000070a000000020202";

        let input = hex::decode(INPUT_HEX).expect("invalid input hex");

        let (rem, parsed) = Transfer::from_bytes(&input).expect("couldn't parse transfer");
        assert_eq!(rem.len(), 0);

        let expected = Transfer {
            source: (Curve::Bip32Ed25519, arrayref::array_ref!(input, 1, 20)),
            fee: Zarith {
                is_negative: None,
                bytes: &input[21..23],
            },
            counter: Zarith {
                is_negative: None,
                bytes: &input[23..24],
            },
            gas_limit: Zarith {
                is_negative: None,
                bytes: &input[24..25],
            },
            storage_limit: Zarith {
                is_negative: None,
                bytes: &input[25..26],
            },
            amount: Zarith {
                is_negative: None,
                bytes: &input[26..28],
            },
            destination: ContractID::Implicit(
                Curve::Bip32Ed25519,
                arrayref::array_ref!(input, 28 + 2, 20),
            ),
            parameters: Some(Parameters {
                entrypoint: Entrypoint::Do,
                michelson: &input[56..],
            }),
        };

        assert_eq!(parsed, expected);
    }
}
