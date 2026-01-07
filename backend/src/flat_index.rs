use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

const MAGIC: &[u8; 4] = b"RVIX";
const VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct FlatIndex {
    index_path: String,
    meta_path: String,
    dim: usize,
}

impl FlatIndex {
    pub fn open_or_create(index_path: impl Into<String>, dim: usize) -> io::Result<Self> {
        let index_path = index_path.into();
        let meta_path = format!("{}.meta", index_path);

        // ensure dir exists
        if let Some(parent) = Path::new(&index_path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        // create file if not exist + write header
        if !Path::new(&index_path).exists() {
            let mut f = File::create(&index_path)?;
            f.write_all(MAGIC)?;
            f.write_u32::<LittleEndian>(VERSION)?;
            f.write_u32::<LittleEndian>(dim as u32)?;
            f.flush()?;

            // init meta
            if !Path::new(&meta_path).exists() {
                Self::write_next_id(&meta_path, 1)?;
            }
        } else {
            // validate header
            let mut f = File::open(&index_path)?;
            let mut magic = [0u8; 4];
            f.read_exact(&mut magic)?;
            if &magic != MAGIC {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad index magic",
                ));
            }
            let ver = f.read_u32::<LittleEndian>()?;
            if ver != VERSION {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported index version",
                ));
            }
            let file_dim = f.read_u32::<LittleEndian>()? as usize;
            if file_dim != dim {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "dim mismatch with existing index",
                ));
            }

            if !Path::new(&meta_path).exists() {
                // if meta missing, rebuild next_id by scanning record count
                let next_id = Self::scan_next_id(&index_path, dim)?;
                Self::write_next_id(&meta_path, next_id)?;
            }
        }

        Ok(Self {
            index_path,
            meta_path,
            dim,
        })
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    pub fn append(&self, vec: &[f32]) -> io::Result<u64> {
        if vec.len() != self.dim {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "dimension mismatch",
            ));
        }

        let id = Self::read_next_id(&self.meta_path)?;

        // append record
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.index_path)?;
        let mut w = BufWriter::new(f);

        w.write_u64::<LittleEndian>(id)?;
        for &v in vec {
            w.write_f32::<LittleEndian>(v)?;
        }
        w.flush()?;

        // bump id
        Self::write_next_id(&self.meta_path, id + 1)?;

        Ok(id)
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> io::Result<Vec<(u64, f32)>> {
        if query.len() != self.dim {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "dimension mismatch",
            ));
        }
        if top_k == 0 {
            return Ok(vec![]);
        }

        let f = File::open(&self.index_path)?;
        let mut r = BufReader::new(f);

        // skip header: magic(4) + ver(4) + dim(4)
        r.seek(SeekFrom::Start(12))?;

        // cosine helpers
        fn dot(a: &[f32], b: &[f32]) -> f32 {
            a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
        }
        fn norm(a: &[f32]) -> f32 {
            dot(a, a).sqrt()
        }
        fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
            let na = norm(a);
            let nb = norm(b);
            if na == 0.0 || nb == 0.0 {
                1.0
            } else {
                let sim = dot(a, b) / (na * nb);
                1.0 - sim
            }
        }

        // maintain top_k by distance asc
        let mut best: Vec<(u64, f32)> = Vec::with_capacity(top_k);

        loop {
            // read id
            let id = match r.read_u64::<LittleEndian>() {
                Ok(v) => v,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            // read vec
            let mut vec = vec![0f32; self.dim];
            for i in 0..self.dim {
                vec[i] = r.read_f32::<LittleEndian>()?;
            }

            let dist = cosine_distance(query, &vec);

            // insert into best
            if best.len() < top_k {
                best.push((id, dist));
                best.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            } else if let Some(&(_, worst_dist)) = best.last() {
                if dist < worst_dist {
                    best.pop();
                    best.push((id, dist));
                    best.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                }
            }
        }

        Ok(best)
    }

    fn read_next_id(meta_path: &str) -> io::Result<u64> {
        let mut f = File::open(meta_path)?;
        let mut buf = [0u8; 8];
        f.read_exact(&mut buf)?;
        let mut c = io::Cursor::new(buf);
        Ok(c.read_u64::<LittleEndian>()?)
    }

    fn write_next_id(meta_path: &str, next_id: u64) -> io::Result<()> {
        if let Some(parent) = Path::new(meta_path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut f = File::create(meta_path)?;
        f.write_u64::<LittleEndian>(next_id)?;
        f.flush()?;
        Ok(())
    }

    fn scan_next_id(index_path: &str, dim: usize) -> io::Result<u64> {
        let f = File::open(index_path)?;
        let mut r = BufReader::new(f);
        r.seek(SeekFrom::Start(12))?;

        let record_bytes = 8u64 + (dim as u64) * 4u64;
        let mut count: u64 = 0;

        loop {
            // try read id
            let _id = match r.read_u64::<LittleEndian>() {
                Ok(v) => v,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            // skip vec quickly by reading raw bytes
            let mut skip = vec![0u8; (dim * 4) as usize];
            r.read_exact(&mut skip)?;

            count += 1;

            // (optional) sanity: if seekable, could use file size math, but keep simple
            let _ = record_bytes;
        }

        Ok(count + 1) // since id is sequential, next_id == record_count
    }
}
