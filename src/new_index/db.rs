use rocksdb;

use std::path::Path;

use crate::util::Bytes;

pub struct DBRow {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

pub struct DB {
    db: rocksdb::DB,
}

impl DB {
    pub fn open(path: &Path) -> DB {
        debug!("opening DB at {:?}", path);
        let mut db_opts = rocksdb::Options::default();
        db_opts.create_if_missing(true);
        db_opts.set_max_open_files(-1); // TODO: make sure to `ulimit -n` this process correctly
        db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Level);
        db_opts.set_compression_type(rocksdb::DBCompressionType::Snappy);
        db_opts.set_target_file_size_base(256 << 20);
        db_opts.set_write_buffer_size(256 << 20);
        db_opts.set_disable_auto_compactions(true); // for initial bulk load

        // db_opts.set_advise_random_on_open(???);
        db_opts.set_compaction_readahead_size(1 << 20);

        // let mut block_opts = rocksdb::BlockBasedOptions::default();
        // block_opts.set_block_size(???);

        let db = rocksdb::DB::open(&db_opts, path).expect("failed to open RocksDB");
        DB { db }
    }

    pub fn compact_all(&self) {
        // TODO: make sure this doesn't fail silently
        self.db.compact_range(None, None);
    }

    pub fn scan(&self, prefix: &[u8]) -> Vec<DBRow> {
        let mode = rocksdb::IteratorMode::From(prefix, rocksdb::Direction::Forward);
        self.db
            .iterator(mode)
            .take_while(|(key, _)| key.starts_with(prefix))
            .map(|(k, v)| DBRow {
                key: k.into_vec(),
                value: v.into_vec(),
            })
            .collect()
    }

    pub fn write(&self, mut rows: Vec<DBRow>) {
        trace!("writing {} rows to {:?}", rows.len(), self.db);
        rows.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        let mut batch = rocksdb::WriteBatch::default();
        for row in rows {
            batch.put(&row.key, &row.value).unwrap();
        }
        let mut opts = rocksdb::WriteOptions::new();
        opts.set_sync(false);
        opts.disable_wal(true);
        self.db.write_opt(batch, &opts).unwrap();
    }

    pub fn get(&self, key: &[u8]) -> Option<Bytes> {
        self.db.get(key).unwrap().map(|v| v.to_vec())
    }
}
