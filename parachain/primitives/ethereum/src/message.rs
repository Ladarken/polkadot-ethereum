use ethereum_types::{H160, U256};
use ethabi::{Event, Param, ParamKind, Token};

use crate::log::Log;

static EVENT_ABI: &'static Event = &Event {
	signature: "AppEvent(uint256,bytes)",
	inputs: &[
		Param { kind: ParamKind::Uint(256), indexed: false },
		Param { kind: ParamKind::Bytes, indexed: false },
	],
	anonymous: false
};

static PAYLOAD_ABI: &'static [ParamKind] = &[
	// sender address
	ParamKind::Address,
	// recipient address (Substrate)
	ParamKind::FixedBytes(32),
	// Token contract address
	ParamKind::Address,
	// Amount
	ParamKind::Uint(256),
	// Nonce
	ParamKind::Uint(256),
];

#[derive(Debug)]
pub enum DecodeError {
	// Unexpected RLP data
	InvalidRLP(rlp::DecoderError),
	// Data does not match expected ABI
	InvalidData(ethabi::Error),
	// Expect "SendETH" or "SendERC20"
	InvalidTag,
	// Expected valid hex address
	InvalidAddress,
	// Invalid message payload
	InvalidPayload,
}

impl From<rlp::DecoderError> for DecodeError {
	fn from(err: rlp::DecoderError) -> Self {
		DecodeError::InvalidRLP(err)
	}
}

impl From<ethabi::Error> for DecodeError {
	fn from(err: ethabi::Error) -> Self {
		DecodeError::InvalidData(err)
	}
}

#[derive(Debug)]
enum Tag {
	SendETH = 0,
	SendERC20 = 1
}

const TAG_SENDETH: u8 = Tag::SendETH as u8;
const TAG_SENDERC20: u8 = Tag::SendERC20 as u8;

struct Payload {
    sender: H160,
    recipient: [u8; 32],
    token: H160,
    amount: U256,
    nonce: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Message {
	SendETH {
		sender: H160,
		recipient: [u8; 32],
		amount: U256,
		nonce: u64,
	},
	SendERC20 {
		sender: H160,
		recipient: [u8; 32],
		token: H160,
		amount: U256,
		nonce: u64,
	}
}

impl Message {

	pub fn decode(log: Log) -> Result<Self, DecodeError> {
		let tokens = EVENT_ABI.decode(log.topics, log.data)?;
	
		let mut tokens_iter = tokens.iter();
	
		// extract message tag ("sendETH" or "sendERC20")
		let tag = match tokens_iter.next().ok_or(DecodeError::InvalidPayload)? {
			Token::Uint(value) => value.low_u32() as u8,
			_ => return Err(DecodeError::InvalidPayload)
		};
		
		// extract ABI-encoded message payload
		let payload = match tokens_iter.next().ok_or(DecodeError::InvalidPayload)? {
			Token::Bytes(bytes) => Self::decode_payload(&bytes)?,
			_ => return Err(DecodeError::InvalidPayload)
        };

		match tag {
			TAG_SENDETH => {
				Ok(Message::SendETH {
					sender: payload.sender,
					recipient: payload.recipient,
					amount: payload.amount,
					nonce: payload.nonce,
				})
			},
			TAG_SENDERC20 => {
				Ok(Message::SendERC20 {
					sender: payload.sender,
					recipient: payload.recipient,
					token: payload.token,
					amount: payload.amount,
					nonce: payload.nonce,
				})
			}
			_ => { return Err(DecodeError::InvalidPayload) }
        }

    }

    fn decode_payload(data: &[u8]) -> Result<Payload, DecodeError> {

		let tokens = ethabi::decode(PAYLOAD_ABI, &data)?;
		let mut iter = tokens.iter();
	
		let sender = match iter.next().ok_or(DecodeError::InvalidPayload)? {
			Token::Address(address) => *address,
			_ => return Err(DecodeError::InvalidPayload)
        };
	
		let recipient: [u8; 32] = match iter.next().ok_or(DecodeError::InvalidPayload)? {
			Token::FixedBytes(bytes) => { 
				if bytes.len() != 32 {
					return Err(DecodeError::InvalidPayload)
				}
				let mut dst: [u8; 32] = [0; 32];
				dst.copy_from_slice(&bytes);
				dst.clone()			
			}
			_ => return Err(DecodeError::InvalidPayload)
		};
	
		let token = match iter.next().ok_or(DecodeError::InvalidPayload)? {
			Token::Address(address) => *address,
			_ => return Err(DecodeError::InvalidPayload)
		};
	
		let amount = match iter.next().ok_or(DecodeError::InvalidPayload)? {
			Token::Uint(amount) => *amount,
			_ => return Err(DecodeError::InvalidPayload)
		};
	
		let nonce = match iter.next().ok_or(DecodeError::InvalidPayload)? {
			Token::Uint(value) => value.low_u64(),
			_ => return Err(DecodeError::InvalidPayload)
        };

        Ok(Payload {
            sender,
            recipient,
            token,
            amount,
            nonce,
        })
    }
}


#[cfg(test)]
mod tests {
    use std::io::prelude::*;
	use std::fs::File;
	use std::io::BufReader;
    use super::*;
    use hex::FromHex;
    use std::path::PathBuf;

    fn fixture_path() -> PathBuf {
        [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "log.rlp"].iter().collect()
    }

    fn recipient(hexaddr: &str) -> [u8; 32] {
        let mut buf: [u8; 32] = [0; 32];
        let bytes: Vec<u8> = hexaddr.from_hex().unwrap();
        buf.clone_from_slice(&bytes);
        buf
    }

    #[test]
    fn test_decode() {        
        let mut reader = BufReader::new(File::open(fixture_path()).unwrap());
		let mut data: Vec<u8> = Vec::new();
		reader.read_to_end(&mut data).unwrap();

		let log: Log = rlp::decode(&data).unwrap();
        assert_eq!(Message::decode(log).unwrap(),
            Message::SendETH {
                sender: "cffeaaf7681c89285d65cfbe808b80e502696573".parse().unwrap(),
                recipient: recipient("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"),
                amount: 10.into(), nonce: 7
            }
        );
    }
}
