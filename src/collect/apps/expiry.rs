//! Reads a certificate's expiry date from its DER encoding (RFC 5280),
//! skipping every other field:
//!
//! ```text
//! Certificate     SEQUENCE { tbsCertificate, signatureAlgorithm, signature }
//! tbsCertificate  SEQUENCE { [0] version OPTIONAL, serialNumber, signature,
//!                            issuer, validity, ... }
//! validity        SEQUENCE { notBefore Time, notAfter Time }
//! ```

const SEQUENCE: u8 = 0x30;
const INTEGER: u8 = 0x02;
const VERSION: u8 = 0xA0;
const UTC_TIME: u8 = 0x17;
const GENERALIZED_TIME: u8 = 0x18;

/// `notAfter` as Unix seconds, or `None` for anything malformed.
pub(super) fn not_after(der: &[u8]) -> Option<i64> {
    let certificate = expect(&mut Reader(der), SEQUENCE)?;
    let tbs = expect(&mut Reader(certificate), SEQUENCE)?;
    let mut fields = Reader(tbs);
    let (mut tag, _) = fields.next()?;
    if tag == VERSION {
        (tag, _) = fields.next()?;
    }
    if tag != INTEGER {
        return None;
    }
    fields.next()?; // signature algorithm
    fields.next()?; // issuer
    let mut validity = Reader(expect(&mut fields, SEQUENCE)?);
    validity.next()?; // notBefore
    let (tag, text) = validity.next()?;
    time(tag, text)
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    /// The next element's tag and content.
    fn next(&mut self) -> Option<(u8, &'a [u8])> {
        let (&tag, rest) = self.0.split_first()?;
        let (&first, rest) = rest.split_first()?;
        let (len, rest) = if first < 0x80 {
            (usize::from(first), rest)
        } else {
            let n = usize::from(first & 0x7f);
            if !(1..=4).contains(&n) || rest.len() < n {
                return None;
            }
            let len = rest[..n]
                .iter()
                .fold(0, |len, &b| len << 8 | usize::from(b));
            (len, &rest[n..])
        };
        if rest.len() < len {
            return None;
        }
        let (content, rest) = rest.split_at(len);
        self.0 = rest;
        Some((tag, content))
    }
}

fn expect<'a>(reader: &mut Reader<'a>, tag: u8) -> Option<&'a [u8]> {
    reader
        .next()
        .filter(|&(t, _)| t == tag)
        .map(|(_, content)| content)
}

/// `YYMMDDHHMMSSZ` (UTCTime, years 1950..=2049) or `YYYYMMDDHHMMSSZ`
/// (GeneralizedTime).
fn time(tag: u8, text: &[u8]) -> Option<i64> {
    let digits = text.strip_suffix(b"Z")?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let number = |range: std::ops::Range<usize>| {
        digits[range]
            .iter()
            .fold(0, |n, &d| n * 10 + i64::from(d - b'0'))
    };
    let (year, rest) = match (tag, digits.len()) {
        (UTC_TIME, 12) => {
            let yy = number(0..2);
            (if yy < 50 { 2000 + yy } else { 1900 + yy }, 2)
        }
        (GENERALIZED_TIME, 14) => (number(0..4), 4),
        _ => return None,
    };
    let [month, day, hour, minute, second] =
        [0, 2, 4, 6, 8].map(|offset| number(rest + offset..rest + offset + 2));
    let valid = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60;
    valid.then(|| days_from_civil(year, month, day) * 86400 + hour * 3600 + minute * 60 + second)
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        match content.len() {
            len @ 0..0x80 => out.push(len as u8),
            len => {
                out.push(0x82);
                out.extend_from_slice(&(len as u16).to_be_bytes());
            }
        }
        out.extend_from_slice(content);
        out
    }

    fn certificate(with_version: bool, not_after: Vec<u8>) -> Vec<u8> {
        let mut tbs = Vec::new();
        if with_version {
            tbs.extend(tlv(VERSION, &tlv(INTEGER, &[2])));
        }
        tbs.extend(tlv(INTEGER, &[1]));
        tbs.extend(tlv(SEQUENCE, &[])); // signature algorithm
        tbs.extend(tlv(SEQUENCE, &[0; 200])); // issuer, long enough for 0x82 lengths
        let mut validity = tlv(UTC_TIME, b"240101000000Z");
        validity.extend(not_after);
        tbs.extend(tlv(SEQUENCE, &validity));
        tbs.extend(tlv(SEQUENCE, &[])); // subject
        let mut cert = tlv(SEQUENCE, &tbs);
        cert.extend(tlv(SEQUENCE, &[])); // signature algorithm
        tlv(SEQUENCE, &cert)
    }

    #[test]
    fn reads_not_after() {
        // 2025-03-01T12:34:56Z
        let expected = Some(1_740_832_496);
        let utc = certificate(true, tlv(UTC_TIME, b"250301123456Z"));
        assert_eq!(not_after(&utc), expected);
        let generalized = certificate(false, tlv(GENERALIZED_TIME, b"20250301123456Z"));
        assert_eq!(not_after(&generalized), expected);
    }

    #[test]
    fn handles_utc_time_centuries() {
        assert_eq!(time(UTC_TIME, b"500101000000Z"), Some(-631_152_000));
        assert_eq!(time(UTC_TIME, b"491231235959Z"), Some(2_524_607_999));
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!(not_after(b""), None);
        assert_eq!(not_after(&[SEQUENCE, 0x85, 1, 2, 3, 4, 5]), None);
        assert_eq!(
            not_after(&certificate(true, tlv(UTC_TIME, b"251301000000Z"))),
            None
        );
        assert_eq!(
            not_after(&certificate(true, tlv(UTC_TIME, b"250301123456"))),
            None
        );
        let mut truncated = certificate(true, tlv(UTC_TIME, b"250301123456Z"));
        truncated.truncate(truncated.len() - 10);
        assert_eq!(not_after(&truncated), None);
    }
}
