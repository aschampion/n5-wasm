use futures;
use js_sys;
use n5;
use serde_json;
use wasm_bindgen;
use wasm_bindgen_futures;
use web_sys;

mod utils;

use std::io::{
    Error,
    ErrorKind,
};

use js_sys::Promise;
use futures::{future::{self, LocalBoxFuture}, Future, FutureExt, TryFutureExt};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use n5::prelude::*;
use n5::{data_type_match, data_type_rstype_replace};


pub mod http_fetch;


pub trait N5PromiseReader {
    /// Get the N5 specification version of the container.
    fn get_version(&self) -> Promise;

    fn get_dataset_attributes(&self, path_name: &str) -> Promise;

    fn exists(&self, path_name: &str) -> Promise;

    fn dataset_exists(&self, path_name: &str) -> Promise;

    fn read_block(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise;

    fn list_attributes(&self, path_name: &str) -> Promise;
}

impl<T> N5PromiseReader for T where T: N5AsyncReader {
    fn get_version(&self) -> Promise {
        let to_return = self.get_version()
            .map_ok(|v| JsValue::from(wrapped::Version(v)));

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn get_dataset_attributes(&self, path_name: &str) -> Promise {
        let to_return = self.get_dataset_attributes(path_name)
            .map_ok(|da| JsValue::from(wrapped::DatasetAttributes(da)));

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn exists(&self, path_name: &str) -> Promise {
        let to_return = self.exists(path_name)
            .map_ok(JsValue::from);

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn dataset_exists(&self, path_name: &str) -> Promise {
        let to_return = self.dataset_exists(path_name)
            .map_ok(JsValue::from);

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn read_block(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise {

        data_type_match! {
            data_attrs.0.get_data_type(),
            future_to_promise(map_future_error_wasm(
                self.read_block::<RsType>(path_name, &data_attrs.0, grid_position.into())
                    .map_ok(|maybe_block| JsValue::from(
                        maybe_block.map(<RsType as VecBlockMonomorphizerReflection>::MONOMORPH::from)))))
        }
    }

    fn list_attributes(
        &self,
        path_name: &str,
    ) -> Promise {

        // TODO: Superfluous conversion from JSON to JsValue to serde to JsValue.
        let to_return = self.list_attributes(path_name)
            .map_ok(|v| JsValue::from_serde(&v).unwrap());

        future_to_promise(map_future_error_wasm(to_return))
    }
}


pub trait N5PromiseEtagReader {
    fn block_etag(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise;

    fn read_block_with_etag(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise;
}

impl<T> N5PromiseEtagReader for T where T: N5AsyncEtagReader {
    fn block_etag(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise {
        let to_return = self.block_etag(path_name, &data_attrs.0, grid_position.into())
            .map_ok(JsValue::from);

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn read_block_with_etag(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise {

        data_type_match! {
            data_attrs.0.get_data_type(),
            future_to_promise(map_future_error_wasm(
                self.read_block_with_etag::<RsType>(path_name, &data_attrs.0, grid_position.into())
                    .map_ok(|maybe_block| JsValue::from(
                        maybe_block.map(<RsType as VecBlockMonomorphizerReflection>::MONOMORPH::from)))))
        }
    }
}


/// This trait exists to preserve type information between calls (rather than
/// erasing it with `Promise`) and for easier potential future compatibility
/// with an N5 core async trait.
pub trait N5AsyncReader {
    fn get_version(&self) -> LocalBoxFuture<'static, Result<n5::Version, Error>>;

    fn get_dataset_attributes(&self, path_name: &str) ->
        LocalBoxFuture<'static, Result<n5::DatasetAttributes, Error>>;

    fn exists(&self, path_name: &str) -> LocalBoxFuture<'static, Result<bool, Error>>;

    fn dataset_exists(&self, path_name: &str) -> LocalBoxFuture<'static, Result<bool, Error>> {
        futures::future::try_join(
            self.exists(path_name),
            self.get_dataset_attributes(path_name)
                    .map_ok(|_| true)
                    .or_else(|_| futures::future::ok(false))
            )
            .map_ok(|(exists, has_attr)| exists && has_attr)
            .boxed_local()
    }

    fn read_block<T>(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: GridCoord,
    ) -> LocalBoxFuture<'static, Result<Option<VecDataBlock<T>>, Error>>
            where VecDataBlock<T>: DataBlock<T>,
                  T: ReflectedType + 'static;

    fn list(&self, path_name: &str) -> LocalBoxFuture<'static, Result<Vec<String>, Error>>;

    fn list_attributes(&self, path_name: &str) -> LocalBoxFuture<'static, Result<serde_json::Value, Error>>;
}


pub trait N5AsyncEtagReader {
    fn block_etag(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: GridCoord,
    ) -> LocalBoxFuture<'static, Result<Option<String>, Error>>;

    fn read_block_with_etag<T>(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: GridCoord,
    ) -> LocalBoxFuture<'static, Result<Option<(VecDataBlock<T>, Option<String>)>, Error>>
            where VecDataBlock<T>: DataBlock<T>,
                  T: ReflectedType + 'static;
}


fn map_future_error_rust<F: Future<Output=Result<T, JsValue>>, T>(future: F)
        -> impl Future<Output=Result<T, Error>> {
    future.map_err(convert_jsvalue_error)
}

fn map_future_error_wasm<F: Future<Output=Result<T, Error>>, T>(future: F)
        -> impl Future<Output=Result<T, JsValue>> {
    future.map_err(|error| {
        let js_error = js_sys::Error::new(&format!("{:?}", error));
        JsValue::from(js_error)
    })
}

fn convert_jsvalue_error(error: JsValue) -> Error {
    Error::new(std::io::ErrorKind::Other, format!("{:?}", error))
}


pub mod wrapped {
    use super::*;

    #[wasm_bindgen]
    pub struct Version(pub(crate) n5::Version);

    #[wasm_bindgen]
    impl Version {
        pub fn to_string(&self) -> String {
            self.0.to_string()
        }
    }

    #[wasm_bindgen]
    pub struct DatasetAttributes(pub(crate) n5::DatasetAttributes);

    #[wasm_bindgen]
    impl DatasetAttributes {
        pub fn get_dimensions(&self) -> Vec<i64> {
            self.0.get_dimensions().to_owned()
        }

        pub fn get_block_size(&self) -> Vec<i32> {
            self.0.get_block_size().to_owned()
        }

        pub fn get_data_type(&self) -> String {
            self.0.get_data_type().to_string()
        }

        pub fn get_compression(&self) -> String {
            self.0.get_compression().to_string()
        }

        pub fn get_ndim(&self) -> usize {
            self.0.get_ndim()
        }

        /// Get the total number of elements possible given the dimensions.
        pub fn get_num_elements(&self) -> usize {
            self.0.get_num_elements()
        }

        /// Get the total number of elements possible in a block.
        pub fn get_block_num_elements(&self) -> usize {
            self.0.get_block_num_elements()
        }
    }
}

trait VecBlockMonomorphizerReflection {
    type MONOMORPH;
}

macro_rules! data_block_monomorphizer {
    ($d_name:ident, $d_type:ty) => {
        #[wasm_bindgen]
        pub struct $d_name(VecDataBlock<$d_type>, Option<String>);

        impl VecBlockMonomorphizerReflection for $d_type {
            type MONOMORPH = $d_name;
        }

        impl From<VecDataBlock<$d_type>> for $d_name {
            fn from(block: VecDataBlock<$d_type>) -> Self {
                $d_name(block, None)
            }
        }

        impl From<(VecDataBlock<$d_type>, Option<String>)> for $d_name {
            fn from((block, etag): (VecDataBlock<$d_type>, Option<String>)) -> Self {
                $d_name(block, etag)
            }
        }

        #[wasm_bindgen]
        impl $d_name {
            pub fn get_size(&self) -> Vec<i32> {
                self.0.get_size().to_owned()
            }

            pub fn get_grid_position(&self) -> Vec<i64> {
                self.0.get_grid_position().to_owned()
            }

            pub fn get_data(&self) -> Vec<$d_type> {
                self.0.get_data().to_owned()
            }

            pub fn into_data(self) -> Vec<$d_type> {
                self.0.into()
            }

            pub fn get_num_elements(&self) -> i32 {
                self.0.get_num_elements()
            }

            pub fn get_etag(&self) -> Option<String> {
                self.1.to_owned()
            }
        }
    }
}

data_block_monomorphizer!(VecDataBlockUINT8,  u8);
data_block_monomorphizer!(VecDataBlockUINT16, u16);
data_block_monomorphizer!(VecDataBlockUINT32, u32);
data_block_monomorphizer!(VecDataBlockUINT64, u64);
data_block_monomorphizer!(VecDataBlockINT8,  i8);
data_block_monomorphizer!(VecDataBlockINT16, i16);
data_block_monomorphizer!(VecDataBlockINT32, i32);
data_block_monomorphizer!(VecDataBlockINT64, i64);
data_block_monomorphizer!(VecDataBlockFLOAT32, f32);
data_block_monomorphizer!(VecDataBlockFLOAT64, f64);
