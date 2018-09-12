extern crate cfg_if;
extern crate futures;
extern crate js_sys;
extern crate n5;
extern crate serde_json;
extern crate wasm_bindgen;
extern crate wasm_bindgen_futures;
extern crate web_sys;

mod utils;

use std::fmt::Write;
use std::io::{
    Error,
    ErrorKind,
};
use std::str::FromStr;

use js_sys::{
    ArrayBuffer,
    Promise,
};
use futures::{future, Future};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{Request, RequestInit, RequestMode, Response, Window};

use n5::prelude::*;


const ATTRIBUTES_FILE: &str = "attributes.json";


pub trait N5PromiseReader {
    /// Get the N5 specification version of the container.
    fn get_version(&self) -> Promise;

    fn exists(&self, path_name: &str) -> Promise;

    fn get_dataset_attributes(&self, path_name: &str) -> Promise;

    fn read_block(
        &self,
        path_name: &str,
        data_attrs: &MyDatasetAttributes,
        grid_position: Vec<i64>
    ) -> Promise;
}

impl<T> N5PromiseReader for T where T: N5AsyncReader {
    fn get_version(&self) -> Promise {
        let to_return = self.get_version()
            .map(JsValue::from);

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn exists(&self, path_name: &str) -> Promise {
        let to_return = self.exists(path_name)
            .map(JsValue::from);

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn get_dataset_attributes(&self, path_name: &str) -> Promise {
        let to_return = self.get_dataset_attributes(path_name)
            .map(JsValue::from);

        future_to_promise(map_future_error_wasm(to_return))
    }

    fn read_block(
        &self,
        path_name: &str,
        data_attrs: &MyDatasetAttributes,
        grid_position: Vec<i64>
    ) -> Promise {
        let to_return = match data_attrs.0.get_data_type() {
            DataType::UINT8 => {
                self.read_block::<u8>(path_name, &data_attrs.0, grid_position)
                    .map(|maybe_block| {
                        JsValue::from(maybe_block.map(VecDataBlockUINT8))
                    })
            }
            _ => unimplemented!()
        };

        future_to_promise(map_future_error_wasm(to_return))
    }
}

#[wasm_bindgen]
pub struct MyVersion(n5::Version);

#[wasm_bindgen]
impl MyVersion {
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[wasm_bindgen]
pub struct MyDatasetAttributes(DatasetAttributes);

#[wasm_bindgen]
pub struct VecDataBlockUINT8(VecDataBlock<u8>);

// This trait exists to preserve type information between calls (rather than
// erasing it with `Promise`) and for easier potential future compatibility
// with an N5 core async trait.
pub trait N5AsyncReader {
    fn get_version(&self) -> Box<Future<Item = MyVersion, Error = Error>>;

    fn exists(&self, path_name: &str) -> Box<Future<Item = bool, Error = Error>>;

    fn get_dataset_attributes(&self, path_name: &str) ->
        Box<Future<Item = MyDatasetAttributes, Error = Error>>;

    fn read_block<T>(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: Vec<i64>
    ) -> Box<Future<Item = Option<VecDataBlock<T>>, Error = Error>>
            where DataType: n5::DataBlockCreator<T>,
                  VecDataBlock<T>: DataBlock<T>,
                  T: 'static;
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct N5HTTPFetch {
    base_path: String,
}

impl N5HTTPFetch {
    fn fetch(&self, path_name: &str) -> JsFuture {
        let mut request_options = RequestInit::new();
        request_options.method("GET");
        request_options.mode(RequestMode::Cors);

        let req = Request::new_with_str_and_init(
            &format!("{}/{}", &self.base_path, path_name),
            &request_options).unwrap();

        let req_promise = Window::fetch_with_request(&req);

        JsFuture::from(req_promise)
    }

    fn fetch_json(&self, path_name: &str) -> Box<Future<Item = JsValue, Error = JsValue>> {
        let to_return = self.fetch(path_name).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into()?;

            resp.json()
        }).and_then(|json_value: Promise| {
            JsFuture::from(json_value)
        });

        Box::new(to_return)
    }

    fn get_attributes(&self, path_name: &str) -> Box<Future<Item = serde_json::Value, Error = Error>> {
        let to_return = self
            .fetch_json(&format!("{}/{}", path_name, ATTRIBUTES_FILE))
            .map(|json| json.into_serde().unwrap());

        Box::new(map_future_error_rust(to_return))
    }
}

#[wasm_bindgen]
impl N5HTTPFetch {
    pub fn open(base_path: &str) -> Promise {
        let reader = N5HTTPFetch {
            base_path: base_path.into(),
        };

        let to_return = N5AsyncReader::get_version(&reader).and_then(|version| {

            if !n5::VERSION.is_compatible(&version.0) {
                return future::err(Error::new(ErrorKind::Other, "TODO: Incompatible version"))
            }

            future::ok(JsValue::from(reader))
        });

        future_to_promise(map_future_error_wasm(to_return))
    }
}

// WASM bindings for the N5Reader-mirror trait
#[wasm_bindgen]
impl N5HTTPFetch {
    pub fn get_version(&self) -> Promise {
        N5PromiseReader::get_version(self)
    }

    pub fn exists(&self, path_name: &str) -> Promise {
        N5PromiseReader::exists(self, path_name)
    }

    pub fn get_dataset_attributes(&self, path_name: &str) -> Promise {
        N5PromiseReader::get_dataset_attributes(self, path_name)
    }

    pub fn read_block(
        &self,
        path_name: &str,
        data_attrs: &MyDatasetAttributes,
        grid_position: Vec<i64>
    ) -> Promise {
        N5PromiseReader::read_block(self, path_name, data_attrs, grid_position)
    }
}

fn map_future_error_rust<F: Future<Item = T, Error = JsValue>, T>(future: F) -> impl Future<Item = T, Error = Error> {
    future.map_err(convert_jsvalue_error)
}

fn map_future_error_wasm<F: Future<Item = T, Error = Error>, T>(future: F) -> impl Future<Item = T, Error = JsValue> {
    future.map_err(|error| {
        let js_error = js_sys::Error::new(&format!("uh oh! {:?}", error));
        JsValue::from(js_error)
    })
}

fn convert_jsvalue_error(error: JsValue) -> Error {
    Error::new(std::io::ErrorKind::Other, format!("{:?}", error))
}

// fn array_buffer_to_vec<T>(buff: ArrayBuffer) -> Vec<T>

impl N5AsyncReader for N5HTTPFetch {
    fn get_version(&self) -> Box<Future<Item = MyVersion, Error = Error>> {
        let to_return = self.get_attributes("").map(|attr| {
            MyVersion(n5::Version::from_str(attr
                    .get(n5::VERSION_ATTRIBUTE_KEY)
                    .unwrap()
                    .as_str().unwrap_or("")
                ).unwrap())
        });

        Box::new(to_return)
    }

    fn exists(&self, path_name: &str) -> Box<Future<Item = bool, Error = Error>> {
        let to_return = self.fetch(path_name).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();

            future::ok(resp.ok())
        });

        Box::new(map_future_error_rust(to_return))
    }

    fn get_dataset_attributes(&self, path_name: &str) ->
            Box<Future<Item = MyDatasetAttributes, Error = Error>> {

        let to_return = self
            .fetch_json(&format!("{}/{}", path_name, ATTRIBUTES_FILE))
            .map(|json| {
                let da = json.into_serde();

                if let Err(ref x) = da {Window::alert_with_message(&x.to_string());}

                MyDatasetAttributes(da.unwrap())});

        Box::new(map_future_error_rust(to_return))
    }

    fn read_block<T>(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: Vec<i64>
    ) -> Box<Future<Item = Option<VecDataBlock<T>>, Error = Error>>
            where DataType: n5::DataBlockCreator<T>,
                  VecDataBlock<T>: DataBlock<T>,
                  T: 'static {

        let da2 = data_attrs.clone();

        let mut block_path = String::new();
        for coord in &grid_position {
            write!(block_path, "/{}", coord);
        }
        // let block_path = format!("{}/{}", path_name,
        //     grid_position.iter().map(ToString::to_string).collect::<String>().join("/"));

        let f = self.fetch(&format!("{}{}", path_name, block_path)).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();

            if resp.ok() {
                let to_return = JsFuture::from(resp.array_buffer().unwrap())
                    .map(move |arrbuff_value| {
                        assert!(arrbuff_value.is_instance_of::<ArrayBuffer>());
                        let typebuff: js_sys::Uint8Array = js_sys::Uint8Array::new(&arrbuff_value);

                        let mut buff: Vec<u8> = Vec::with_capacity(typebuff.length() as usize);

                        typebuff.for_each(&mut |byte, _, _| buff.push(byte));

                        Some(<n5::DefaultBlock as n5::DefaultBlockReader<T, &[u8]>>::read_block(
                            &buff,
                            &da2,
                            grid_position).unwrap())
                    });
                future::Either::A(to_return)
            } else  {
                future::Either::B(future::ok(None))
            }
        });

        Box::new(map_future_error_rust(f))
    }
}
