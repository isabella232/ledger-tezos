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

use cfg_if::cfg_if;

use crate::constants::ApduError::{ClaNotSupported, CommandNotAllowed, Success, WrongLength};
use crate::constants::{ApduError, APDU_INDEX_CLA, APDU_INDEX_INS, APDU_MIN_LENGTH};
use crate::handlers::legacy_version::{LegacyGetVersion, LegacyGit};
use crate::handlers::public_key::GetAddress;
use crate::handlers::signing::Sign;
use crate::handlers::version::GetVersion;
use crate::utils::{ApduBufferRead, ApduBufferWrite};

pub const CLA: u8 = 0x80;

#[cfg(feature = "baking")]
use crate::handlers::baking::Baking;

cfg_if! {
    if #[cfg(feature = "baking")] {
        //baking-only legacy instructions
        pub const INS_LEGACY_AUTHORIZE_BAKING: u8 = 0x1;
        pub const INS_LEGACY_RESET: u8 = 0x6;
        pub const INS_LEGACY_QUERY_AUTH_KEY: u8 = 0x7;
        pub const INS_LEGACY_QUERY_MAIN_HWM: u8 = 0x8;
        pub const INS_LEGACY_SETUP: u8 = 0xA;
        pub const INS_LEGACY_QUERY_ALL_HWM: u8 = 0xB;
        pub const INS_LEGACY_DEAUTHORIZE: u8 = 0xC;
        pub const INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE: u8 = 0xD;
        pub const INS_LEGACY_HMAC: u8 = 0xE;

        pub const INS_AUTHORIZE_BAKING: u8 = 0xA1;
        pub const INS_DEAUTHORIZE_BAKING: u8 = 0xAC;
        pub const INS_QUERY_AUTH_KEY: u8 = 0xA7;
        pub const INS_QUERY_AUTH_KEY_WITH_CURVE: u8 = 0xAD;
        pub const INS_BAKER_SIGN: u8 = 0xAF;

        //baking-only legacy imports
        use crate::handlers::hwm::LegacyHWM;

        //baking-only new instructions
    } else if #[cfg(feature = "wallet")] {
        //wallet-only legacy instructions
        pub const INS_LEGACY_SIGN_UNSAFE: u8 = 0x5;

        //wallet-only new instructions
    }
}

//common legacy instructions
pub const INS_LEGACY_GET_VERSION: u8 = 0x0;
pub const INS_LEGACY_GET_PUBLIC_KEY: u8 = 0x2;
pub const INS_LEGACY_PROMPT_PUBLIC_KEY: u8 = 0x3;
pub const INS_LEGACY_SIGN: u8 = 0x4;
pub const INS_LEGACY_GIT: u8 = 0x9;
pub const INS_LEGACY_SIGN_WITH_HASH: u8 = 0xF;

//common new instructions
pub const INS_GET_VERSION: u8 = 0x10;
pub const INS_GET_ADDRESS: u8 = 0x11;
pub const INS_SIGN: u8 = 0x12;

//dev-only
cfg_if! {
    if #[cfg(feature = "dev")] {
        use crate::handlers::dev::{Except, Sha256, Echo};

        pub const INS_DEV_HASH: u8 = 0xF0;
        pub const INS_DEV_EXCEPT: u8 = 0xF1;
        pub const INS_DEV_ECHO_UI: u8 = 0xF2;
    }
}

pub trait ApduHandler {
    fn handle<'apdu>(
        flags: &mut u32,
        apdu_buffer: ApduBufferRead<'apdu>,
    ) -> (ApduBufferWrite<'apdu>, Option<ApduError>);
}

pub fn apdu_dispatch<'apdu>(
    flags: &mut u32,
    apdu_buffer: ApduBufferRead<'apdu>,
) -> (ApduBufferWrite<'apdu>, Option<ApduError>) {
    *flags = 0;

    if apdu_buffer.cla() != CLA {
        return (apdu_buffer.write(), Some(ClaNotSupported));
    }

    let ins = apdu_buffer.ins();

    // Reference for legacy API https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/main.c#L16-L31

    //dev-only instructions
    cfg_if! {
        if #[cfg(feature = "dev")] {
            match ins {
                INS_DEV_HASH => return Sha256::handle(flags, apdu_buffer),
                INS_DEV_EXCEPT => return Except::handle(flags, apdu_buffer),
                INS_DEV_ECHO_UI => return Echo::handle(flags, apdu_buffer),
                _ => {},
            }
        }
    }

    //these are exclusive
    cfg_if! {
        if #[cfg(feature = "baking")] {
            //baking-only instructions
            match ins {
                INS_LEGACY_RESET |
                INS_LEGACY_QUERY_MAIN_HWM |
                INS_LEGACY_QUERY_ALL_HWM => return LegacyHWM::handle(flags, apdu_buffer),

                INS_AUTHORIZE_BAKING |
                INS_DEAUTHORIZE_BAKING |
                INS_QUERY_AUTH_KEY_WITH_CURVE |
                INS_BAKER_SIGN => return Baking::handle(flags apdu_buffer),

                INS_LEGACY_AUTHORIZE_BAKING |
                INS_LEGACY_QUERY_AUTH_KEY |
                INS_LEGACY_SETUP |
                INS_LEGACY_DEAUTHORIZE |
                INS_LEGACY_QUERY_AUTH_KEY_WITH_CURVE |
                INS_LEGACY_HMAC => return (apdu_buffer.write(), Some(CommandNotAllowed)),
                _ => {}
            }
        } else if #[cfg(feature = "wallet")] {
            //wallet-only instructions
            #[allow(clippy::single_match)]
            match ins {
                INS_LEGACY_SIGN_UNSAFE => return Sign::handle(flags, apdu_buffer),
                _ => {}
            }
        }
    }

    //common instructions
    // FIXME: Unify using the trait
    match ins {
        INS_LEGACY_GET_VERSION => LegacyGetVersion::handle(flags, apdu_buffer),

        INS_LEGACY_GET_PUBLIC_KEY | INS_LEGACY_PROMPT_PUBLIC_KEY | INS_GET_ADDRESS => {
            GetAddress::handle(flags, apdu_buffer)
        }

        INS_LEGACY_GIT => LegacyGit::handle(flags, apdu_buffer),

        INS_LEGACY_SIGN | INS_LEGACY_SIGN_WITH_HASH | INS_SIGN => Sign::handle(flags, apdu_buffer),

        INS_GET_VERSION => GetVersion::handle(flags, apdu_buffer),
        _ => (apdu_buffer.write(), Some(CommandNotAllowed)),
    }
}

pub fn handle_apdu(flags: &mut u32, tx: &mut u32, rx: u32, apdu_buffer: &mut [u8]) {
    crate::sys::zemu_log_stack("handle_apdu\x00");

    let n = {
        //construct reader
        let read = ApduBufferRead::new(apdu_buffer, rx);
        match read {
            Ok(r) => {
                //pass to dispatch
                let (write, mby_err) = apdu_dispatch(flags, r);

                //close with error (if an error was given)
                // or success (if no error)
                write.close(mby_err)
            }
            //if failed to construct read, then it's too small
            Err(_) => ApduBufferWrite::new(apdu_buffer).close(Some(ApduError::WrongLength)),
        }
    };

    match n {
        //if we got an error writing the code, then only write at the first 2 bytes
        Err(_) => {
            let err = ApduError::OutputBufferTooSmall as u16;
            apdu_buffer[0..2].copy_from_slice(&err.to_be_bytes()[..]);
            *tx = 2;
        }
        //otherwise set tx to the number returned by the writer
        Ok(n) => {
            *tx = n as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_error_code;
    use crate::constants::ApduError::WrongLength;
    use crate::dispatcher::handle_apdu;
    use std::convert::TryInto;

    #[test]
    fn apdu_too_short() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 0u32;
        let buffer = &mut [0u8; 260];

        handle_apdu(flags, tx, rx, buffer);
        assert_eq!(*tx, 2u32);
        assert_error_code!(*tx, buffer, WrongLength);
    }

    #[test]
    fn apdu_invalid_cla() {
        let flags = &mut 0u32;
        let tx = &mut 0u32;
        let rx = 5u32;
        let buffer = &mut [0u8; 260];

        handle_apdu(flags, tx, rx, buffer);
        assert_eq!(*tx, 2u32);
    }
}
