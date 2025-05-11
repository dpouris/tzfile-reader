use std::fmt::Display;

use anyhow::anyhow;

#[derive(Debug)]
pub struct TzFile {
    pub header: TzFileHeader,
    pub body: TzFileBody,
}

impl TzFile {
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let bytes_count = bytes.len();
        if bytes_count < 44 {
            return Err(anyhow!("invalid header size '{bytes_count}'"));
        }

        let mut header_bytes = [0; 44];
        for (idx, byte) in bytes.iter().take(44).enumerate() {
            header_bytes[idx] = *byte;
        }

        let header = match TzFileHeader::try_from(header_bytes) {
            Ok(header) => header,
            Err(err) => return Err(anyhow!("header is invalid: {}", err)),
        };

        let body = match TzFileBody::from_bytes_and_header(&bytes[44..], &header) {
            Ok(body) => body,
            Err(err) => return Err(anyhow!("invalid bytes for Tz File body: {}", err)),
        };
        Ok(Self { header, body })
    }
}

#[derive(Debug)]
pub struct TzFileHeader {
    magic: String,
    pub version: char, // 0, 2, 3, 4
    _reserved: [u8; 15],

    tzh_ttisutcnt: i32,
    tzh_ttisstdcnt: i32,
    tzh_leapcnt: i32,
    tzh_timecnt: i32,
    tzh_typecnt: i32,
    tzh_charcnt: i32,
}

impl TryFrom<[u8; 44]> for TzFileHeader {
    type Error = anyhow::Error;

    fn try_from(value: [u8; 44]) -> Result<Self, Self::Error> {
        let magic = String::from_utf8_lossy(&value[0..4]).into_owned();
        if magic != "TZif" {
            return Err(anyhow!(
                "invalid TZ Info magic header '{}', expected 'TZif'",
                magic
            ));
        }

        Ok(Self {
            magic,
            version: value[4] as char,
            _reserved: [
                value[5], value[6], value[7], value[8], value[9], value[10], value[11], value[12],
                value[13], value[14], value[15], value[16], value[17], value[18], value[19],
            ],

            tzh_ttisutcnt: i32::from_be_bytes([value[20], value[21], value[22], value[23]]),
            tzh_ttisstdcnt: i32::from_be_bytes([value[24], value[25], value[26], value[27]]),
            tzh_leapcnt: i32::from_be_bytes([value[28], value[29], value[30], value[31]]),
            tzh_timecnt: i32::from_be_bytes([value[32], value[33], value[34], value[35]]),
            tzh_typecnt: i32::from_be_bytes([value[36], value[37], value[38], value[39]]),
            tzh_charcnt: i32::from_be_bytes([value[40], value[41], value[42], value[43]]),
        })
    }
}

impl Display for TzFileHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} V{}", self.magic, self.version)
    }
}

#[derive(Debug)]
pub struct TzFileBody {
    pub tt_trans: Vec<i32>, // be
    pub ttinfo_indices: Vec<u8>,
    pub ttinfo_entries: Vec<TTInfo>, // be
    pub tz_designations: String,     // null terminated strs
    pub leap_pairs: Vec<(i32, i32)>, // be
    pub std_indicators: Vec<bool>,
    pub ut_indicators: Vec<bool>,
}

impl TzFileBody {
    fn from_bytes_and_header(bytes: &[u8], header: &TzFileHeader) -> anyhow::Result<Self> {
        let mut left_idx = 0usize;
        let trans = bytes[left_idx..left_idx + header.tzh_timecnt as usize * size_of::<i32>()]
            .chunks(size_of::<i32>())
            .map(|chunk| i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        left_idx += header.tzh_timecnt as usize * size_of::<i32>();

        let ttinfo_indices = bytes[left_idx..left_idx + header.tzh_timecnt as usize].to_vec();
        left_idx += header.tzh_timecnt as usize;

        let ttinfo_size_unpadded = size_of::<TTInfo>() - 2; // the padding is 2 for ttinfo
        let ttinfo_entries = bytes
            [left_idx..left_idx + header.tzh_typecnt as usize * ttinfo_size_unpadded]
            .chunks(ttinfo_size_unpadded) // each ttinfo struct contains 6 bytes
            .flat_map(TTInfo::from_bytes)
            .collect();
        left_idx += header.tzh_typecnt as usize * ttinfo_size_unpadded;

        let designations =
            String::from_utf8_lossy(&bytes[left_idx..left_idx + header.tzh_charcnt as usize])
                .into_owned();
        // .split(|&b| b == b'\0')
        // .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        // .collect::<Vec<String>>()
        // .join("\0");
        // designations.strip_suffix('\0');
        // designations.pop(); // due to split, the last string in the vec will be an empty string because we split on the null terminator and the last string is split into 2
        left_idx += header.tzh_charcnt as usize;

        let leap_pairs = bytes
            [left_idx..left_idx + header.tzh_leapcnt as usize * size_of::<(i32, i32)>()]
            .chunks(size_of::<(i32, i32)>()) // a pair contains 2,  4-byte values
            .map(|chunk| {
                (
                    i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
                    i32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
                )
            })
            .collect();
        left_idx += header.tzh_leapcnt as usize * size_of::<(i32, i32)>();

        let std_indicators = bytes[left_idx..left_idx + header.tzh_ttisstdcnt as usize]
            .iter()
            .map(|&b| b == 1)
            .collect();
        left_idx += header.tzh_ttisstdcnt as usize;

        let ut_indicators = bytes[left_idx..left_idx + header.tzh_ttisutcnt as usize]
            .iter()
            .map(|&b| b == 1)
            .collect();

        Ok(Self {
            tt_trans: trans,
            ttinfo_indices,
            ttinfo_entries,
            tz_designations: designations,
            leap_pairs,
            std_indicators,
            ut_indicators,
        })
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
#[repr(C)]
pub struct TTInfo {
    pub tt_utoff: i32,
    pub tt_isdst: bool,
    pub tt_desigidx: u8,
}

impl TTInfo {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 6 {
            return Err(anyhow!(
                "invalid number of bytes '{}', expected 6",
                bytes.len()
            ));
        }

        Ok(Self {
            tt_utoff: i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            tt_isdst: bytes[4] == 1,
            tt_desigidx: bytes[5],
        })
    }
}
