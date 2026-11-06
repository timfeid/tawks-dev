//! A vector tile reader for the few OpenMapTiles layers the heist needs on
//! the server: roads, road names and buildings.

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Feature {
    /// 1 point, 2 line, 3 polygon.
    pub kind: u32,
    pub props: HashMap<String, Value>,
    /// Each part as flat x, y pairs in tile units, y down.
    pub parts: Vec<Vec<f64>>,
}

impl Feature {
    pub fn str(&self, key: &str) -> &str {
        self.props.get(key).and_then(Value::as_str).unwrap_or("")
    }

    pub fn num(&self, key: &str) -> Option<f64> {
        self.props.get(key).and_then(Value::as_num)
    }
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub extent: f64,
    pub features: Vec<Feature>,
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn done(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn varint(&mut self) -> Option<u64> {
        let mut out = 0u64;
        let mut shift = 0;
        loop {
            let byte = *self.buf.get(self.pos)?;
            self.pos += 1;
            out |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                return Some(out);
            }
            shift += 7;
            if shift > 63 {
                return None;
            }
        }
    }

    fn bytes(&mut self) -> Option<&'a [u8]> {
        let len = self.varint()? as usize;
        let out = self.buf.get(self.pos..self.pos + len)?;
        self.pos += len;
        Some(out)
    }

    fn skip(&mut self, wire: u64) -> Option<()> {
        match wire {
            0 => {
                self.varint()?;
            }
            1 => self.pos += 8,
            2 => {
                self.bytes()?;
            }
            5 => self.pos += 4,
            _ => return None,
        }
        Some(())
    }
}

fn zigzag(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}

fn packed(bytes: &[u8]) -> Vec<u64> {
    let mut r = Reader { buf: bytes, pos: 0 };
    let mut out = Vec::new();
    while !r.done() {
        match r.varint() {
            Some(v) => out.push(v),
            None => break,
        }
    }
    out
}

fn value(bytes: &[u8]) -> Option<Value> {
    let mut r = Reader { buf: bytes, pos: 0 };
    let mut out = Value::Str(String::new());
    while !r.done() {
        let tag = r.varint()?;
        match tag >> 3 {
            1 => out = Value::Str(String::from_utf8_lossy(r.bytes()?).into_owned()),
            2 => {
                let b = r.buf.get(r.pos..r.pos + 4)?;
                out = Value::Num(f32::from_le_bytes(b.try_into().ok()?) as f64);
                r.pos += 4;
            }
            3 => {
                let b = r.buf.get(r.pos..r.pos + 8)?;
                out = Value::Num(f64::from_le_bytes(b.try_into().ok()?));
                r.pos += 8;
            }
            4 | 5 => out = Value::Num(r.varint()? as f64),
            6 => out = Value::Num(zigzag(r.varint()?) as f64),
            7 => out = Value::Bool(r.varint()? != 0),
            _ => r.skip(tag & 7)?,
        }
    }
    Some(out)
}

pub fn geometry(commands: &[u64]) -> Vec<Vec<f64>> {
    let mut parts: Vec<Vec<f64>> = Vec::new();
    let (mut x, mut y) = (0i64, 0i64);
    let mut i = 0;
    while i < commands.len() {
        let word = commands[i];
        i += 1;
        let cmd = word & 7;
        let count = word >> 3;
        if cmd == 7 {
            if let Some(part) = parts.last_mut()
                && part.len() >= 2
            {
                let (a, b) = (part[0], part[1]);
                part.push(a);
                part.push(b);
            }
            continue;
        }
        for _ in 0..count {
            let (Some(&dx), Some(&dy)) = (commands.get(i), commands.get(i + 1)) else {
                break;
            };
            i += 2;
            x += zigzag(dx);
            y += zigzag(dy);
            if cmd == 1 {
                parts.push(Vec::new());
            }
            if let Some(part) = parts.last_mut() {
                part.push(x as f64);
                part.push(y as f64);
            }
        }
    }
    parts
}

fn layer(bytes: &[u8], wanted: &[&str]) -> Option<Option<Layer>> {
    let mut r = Reader { buf: bytes, pos: 0 };
    let mut name = String::new();
    let mut extent = 4096.0;
    let mut keys = Vec::new();
    let mut values = Vec::new();
    let mut raw = Vec::new();
    while !r.done() {
        let tag = r.varint()?;
        match tag >> 3 {
            1 => {
                name = String::from_utf8_lossy(r.bytes()?).into_owned();
                if !wanted.is_empty() && !wanted.contains(&name.as_str()) {
                    return Some(None);
                }
            }
            2 => raw.push(r.bytes()?),
            3 => keys.push(String::from_utf8_lossy(r.bytes()?).into_owned()),
            4 => values.push(value(r.bytes()?)?),
            5 => extent = r.varint()? as f64,
            _ => r.skip(tag & 7)?,
        }
    }
    let mut features = Vec::new();
    for bytes in raw {
        let mut f = Reader { buf: bytes, pos: 0 };
        let (mut kind, mut tags, mut geom) = (0u64, Vec::new(), Vec::new());
        while !f.done() {
            let tag = f.varint()?;
            match tag >> 3 {
                2 => tags = packed(f.bytes()?),
                3 => kind = f.varint()?,
                4 => geom = packed(f.bytes()?),
                _ => f.skip(tag & 7)?,
            }
        }
        if !(1..=3).contains(&kind) {
            continue;
        }
        let mut props = HashMap::new();
        for pair in tags.chunks(2) {
            if let [k, v] = pair
                && let (Some(k), Some(v)) = (keys.get(*k as usize), values.get(*v as usize))
            {
                props.insert(k.clone(), v.clone());
            }
        }
        features.push(Feature {
            kind: kind as u32,
            props,
            parts: geometry(&geom),
        });
    }
    Some(Some(Layer { name, extent, features }))
}

/// The layers of a tile, only the named ones when `wanted` is not empty.
pub fn decode(buf: &[u8], wanted: &[&str]) -> HashMap<String, Layer> {
    let mut out = HashMap::new();
    let mut r = Reader { buf, pos: 0 };
    while !r.done() {
        let Some(tag) = r.varint() else { break };
        if tag >> 3 == 3 {
            let Some(bytes) = r.bytes() else { break };
            if let Some(Some(l)) = layer(bytes, wanted) {
                out.insert(l.name.clone(), l);
            }
        } else if r.skip(tag & 7).is_none() {
            break;
        }
    }
    out
}

/// Shoelace area in tile units; positive for an outer ring.
pub fn ring_area(ring: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut i = 0;
    while i + 3 < ring.len() {
        sum += ring[i] * ring[i + 3] - ring[i + 2] * ring[i + 1];
        i += 2;
    }
    sum / 2.0
}

#[cfg(test)]
pub mod testing {
    //! Writes small tiles for tests.

    pub enum V {
        S(&'static str),
        N(f64),
    }

    fn varint(out: &mut Vec<u8>, mut n: u64) {
        while n > 127 {
            out.push((n as u8 & 127) | 128);
            n >>= 7;
        }
        out.push(n as u8);
    }

    fn field(out: &mut Vec<u8>, num: u64, bytes: &[u8]) {
        varint(out, (num << 3) | 2);
        varint(out, bytes.len() as u64);
        out.extend_from_slice(bytes);
    }

    fn zig(n: i64) -> u64 {
        ((n << 1) ^ (n >> 63)) as u64
    }

    /// A layer of features, each (kind, props, parts as x, y pairs).
    pub fn tile(layers: &[(&str, Vec<(u32, Vec<(&str, V)>, Vec<Vec<i64>>)>)]) -> Vec<u8> {
        let mut tile = Vec::new();
        for (name, features) in layers {
            let mut keys: Vec<String> = Vec::new();
            let mut vals: Vec<String> = Vec::new();
            let mut layer = Vec::new();
            varint(&mut layer, 15 << 3);
            varint(&mut layer, 2);
            field(&mut layer, 1, name.as_bytes());
            let mut encoded_vals: Vec<Vec<u8>> = Vec::new();
            for (kind, props, parts) in features {
                let mut tags = Vec::new();
                for (k, v) in props {
                    let ki = keys.iter().position(|x| x == k).unwrap_or_else(|| {
                        keys.push(k.to_string());
                        keys.len() - 1
                    });
                    let key = match v {
                        V::S(s) => format!("s{s}"),
                        V::N(n) => format!("n{n}"),
                    };
                    let vi = vals.iter().position(|x| *x == key).unwrap_or_else(|| {
                        vals.push(key.clone());
                        let mut e = Vec::new();
                        match v {
                            V::S(s) => field(&mut e, 1, s.as_bytes()),
                            V::N(n) => {
                                varint(&mut e, (3 << 3) | 1);
                                e.extend_from_slice(&n.to_le_bytes());
                            }
                        }
                        encoded_vals.push(e);
                        vals.len() - 1
                    });
                    varint(&mut tags, ki as u64);
                    varint(&mut tags, vi as u64);
                }
                let mut geom = Vec::new();
                let (mut x, mut y) = (0i64, 0i64);
                for part in parts {
                    let closed = *kind == 3;
                    let pts = part.len() / 2;
                    let count = if closed { pts - 1 } else { pts };
                    varint(&mut geom, (1 << 3) | 1);
                    varint(&mut geom, zig(part[0] - x));
                    varint(&mut geom, zig(part[1] - y));
                    x = part[0];
                    y = part[1];
                    varint(&mut geom, (((count - 1) as u64) << 3) | 2);
                    for i in 1..count {
                        varint(&mut geom, zig(part[i * 2] - x));
                        varint(&mut geom, zig(part[i * 2 + 1] - y));
                        x = part[i * 2];
                        y = part[i * 2 + 1];
                    }
                    if closed {
                        varint(&mut geom, (1 << 3) | 7);
                    }
                }
                let mut feat = Vec::new();
                field(&mut feat, 2, &tags);
                varint(&mut feat, 3 << 3);
                varint(&mut feat, *kind as u64);
                field(&mut feat, 4, &geom);
                field(&mut layer, 2, &feat);
            }
            for k in &keys {
                field(&mut layer, 3, k.as_bytes());
            }
            for v in &encoded_vals {
                field(&mut layer, 4, v);
            }
            varint(&mut layer, 5 << 3);
            varint(&mut layer, 4096);
            field(&mut tile, 3, &layer);
        }
        tile
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{V, tile};
    use super::*;

    #[test]
    fn reads_roads_names_and_buildings() {
        let buf = tile(&[
            (
                "transportation",
                vec![(2, vec![("class", V::S("minor"))], vec![vec![0, 100, 4096, 100]])],
            ),
            (
                "building",
                vec![(
                    3,
                    vec![("render_height", V::N(8.0))],
                    vec![vec![10, 10, 50, 10, 50, 50, 10, 50, 10, 10]],
                )],
            ),
            ("water", vec![(3, vec![], vec![vec![0, 0, 9, 0, 9, 9, 0, 9, 0, 0]])]),
        ]);
        let layers = decode(&buf, &["transportation", "building"]);
        assert_eq!(layers.len(), 2);
        let road = &layers["transportation"].features[0];
        assert_eq!(road.str("class"), "minor");
        assert_eq!(road.parts, vec![vec![0.0, 100.0, 4096.0, 100.0]]);
        let b = &layers["building"].features[0];
        assert_eq!(b.num("render_height"), Some(8.0));
        assert!(ring_area(&b.parts[0]) > 0.0);
        assert!(decode(b"\xff\xff", &[]).is_empty());
    }
}
