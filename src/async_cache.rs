use lru_cache::LruCache;

use super::*;


pub struct N5AsyncCacheReader<N: N5AsyncReader, BT> {
    reader: N,
    dataset: String,
    blocks: LruCache<Vec<i64>, Option<VecDataBlock<BT>>>,
}

impl<N: N5AsyncReader, BT> N5AsyncCacheReader<N, BT> {
    pub fn wrap(
        reader: N,
        dataset: String,
        blocks_capacity: usize,
    ) -> Self {
        Self {
            reader,
            dataset,
            blocks: LruCache::new(blocks_capacity),
        }
    }
}

impl<N: N5AsyncReader, BT: Clone> N5AsyncReader for N5AsyncCacheReader<N, BT>
        where n5::DataType: n5::TypeReflection<BT> {
    fn get_version(&self) -> Box<Future<Item = n5::Version, Error = Error>> {
        self.reader.get_version()
    }

    fn get_dataset_attributes(&self, path_name: &str) ->
        Box<Future<Item = n5::DatasetAttributes, Error = Error>> {

        self.reader.get_dataset_attributes(path_name)
    }

    fn exists(&self, path_name: &str) -> Box<Future<Item = bool, Error = Error>> {
        self.reader.exists(path_name)
    }

    fn read_block<T>(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: Vec<i64>
    ) -> Box<Future<Item = Option<VecDataBlock<T>>, Error = Error>>
            where DataType: n5::DataBlockCreator<T>,
                  VecDataBlock<T>: DataBlock<T>,
                  T: 'static + Clone {

        if path_name == self.dataset {
            // We know BT and T are the same, but assert.
            assert_eq!(<DataType as TypeReflection<BT>>::get_type_variant(),
                       *data_attrs.get_data_type());

            if let Some(block) = self.blocks.get_mut(&grid_position) {
                unsafe {
                    let typed_block = std::mem::transmute::<
                            Option<VecDataBlock<BT>>,
                            Option<VecDataBlock<T>>
                        >(block.clone());

                    Box::new(futures::future::ok(typed_block))
                }
            } else {
                Box::new(self.reader.read_block(path_name, data_attrs, grid_position.clone())
                    .map(|block| {
                        unsafe {
                            let typed_block = std::mem::transmute::<
                                    Option<VecDataBlock<T>>,
                                    Option<VecDataBlock<BT>>
                                >(block.clone());

                            self.blocks.insert(grid_position, typed_block);
                        }

                        block
                    }))
            }
        } else {
            self.reader.read_block(path_name, data_attrs, grid_position)
        }
    }

    fn list(&self, path_name: &str) -> Box<Future<Item = Vec<String>, Error = Error>> {
        self.reader.list(path_name)
    }

    fn list_attributes(&self, path_name: &str) -> Box<Future<Item = serde_json::Value, Error = Error>> {
        self.reader.list_attributes(path_name)
    }
}
