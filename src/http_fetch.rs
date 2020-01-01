use std::fmt::Write;
use std::str::FromStr;

use js_sys::ArrayBuffer;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Request,
    RequestInit,
    RequestMode,
    Response,
};

use super::*;


const ATTRIBUTES_FILE: &str = "attributes.json";


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

        let req_promise = web_sys::window().unwrap().fetch_with_request(&req);

        JsFuture::from(req_promise)
    }

    fn fetch_json(&self, path_name: &str) -> impl Future<Item = JsValue, Error = JsValue> {
        self.fetch(path_name).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into()?;

            resp.json()
        }).and_then(|json_value: Promise| {
            JsFuture::from(json_value)
        })
    }

    fn get_attributes(&self, path_name: &str) -> impl Future<Item = serde_json::Value, Error = Error> {
        let path = self.get_dataset_attributes_path(path_name);
        let to_return = self
            .fetch_json(&path)
            .map(|json| json.into_serde().unwrap());

        map_future_error_rust(to_return)
    }

    fn relative_block_path(&self, path_name: &str, grid_position: &[i64]) -> String {
        let mut block_path = path_name.to_owned();
        for coord in grid_position {
            write!(block_path, "/{}", coord).unwrap();
        }

        block_path
    }

    fn get_dataset_attributes_path(&self, path_name: &str) -> String {
        if path_name.is_empty() {
            ATTRIBUTES_FILE.to_owned()
        } else {
            format!("{}/{}", path_name, ATTRIBUTES_FILE)
        }
    }
}

#[wasm_bindgen]
impl N5HTTPFetch {
    pub fn open(base_path: &str) -> Promise {
        let reader = N5HTTPFetch {
            base_path: base_path.into(),
        };

        let to_return = N5AsyncReader::get_version(&reader).and_then(|version| {

            if !n5::VERSION.is_compatible(&version) {
                return future::err(Error::new(ErrorKind::Other, "TODO: Incompatible version"))
            }

            future::ok(JsValue::from(reader))
        });

        future_to_promise(map_future_error_wasm(to_return))
    }
}

/// Delegations to expose N5PromiseReader trait to WASM.
#[wasm_bindgen]
impl N5HTTPFetch {
    pub fn get_version(&self) -> Promise {
        N5PromiseReader::get_version(self)
    }

    pub fn get_dataset_attributes(&self, path_name: &str) -> Promise {
        N5PromiseReader::get_dataset_attributes(self, path_name)
    }

    pub fn exists(&self, path_name: &str) -> Promise {
        N5PromiseReader::exists(self, path_name)
    }

    pub fn dataset_exists(&self, path_name: &str) -> Promise {
        N5PromiseReader::dataset_exists(self, path_name)
    }

    pub fn read_block(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise {
        N5PromiseReader::read_block(self, path_name, data_attrs, grid_position)
    }

    pub fn list_attributes(&self, path_name: &str) -> Promise {
        N5PromiseReader::list_attributes(self, path_name)
    }

    pub fn block_etag(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>,
    ) -> Promise {
        N5PromiseEtagReader::block_etag(
            self, path_name, data_attrs, grid_position)
    }

    pub fn read_block_with_etag(
        &self,
        path_name: &str,
        data_attrs: &wrapped::DatasetAttributes,
        grid_position: Vec<i64>
    ) -> Promise {
        N5PromiseEtagReader::read_block_with_etag(
            self, path_name, data_attrs, grid_position)
    }
}

impl N5AsyncReader for N5HTTPFetch {
    fn get_version(&self) -> Box<dyn Future<Item = n5::Version, Error = Error>> {
        let to_return = self.get_attributes("")
            .and_then(|attr| {
                let ver = attr.get(n5::VERSION_ATTRIBUTE_KEY)
                    .ok_or_else(|| Error::new(ErrorKind::NotFound, "Not an N5 root"))?;
                Ok(n5::Version::from_str(ver.as_str().unwrap_or("")).unwrap())
            });

        Box::new(to_return)
    }

    fn get_dataset_attributes(&self, path_name: &str) ->
            Box<dyn Future<Item = n5::DatasetAttributes, Error = Error>> {

        let path = self.get_dataset_attributes_path(path_name);
        let to_return = self
            .fetch_json(&path)
            .map(|json| { json.into_serde().unwrap() });

        Box::new(map_future_error_rust(to_return))
    }

    fn exists(&self, path_name: &str) -> Box<dyn Future<Item = bool, Error = Error>> {
        let to_return = self.fetch(path_name).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();

            future::ok(resp.ok())
        });

        Box::new(map_future_error_rust(to_return))
    }

    // Override the default N5AsyncReader impl to not require the GET on the
    // dataset directory path to be 200.
    fn dataset_exists(&self, path_name: &str) -> Box<dyn Future<Item = bool, Error = Error>> {
        let path = self.get_dataset_attributes_path(path_name);
        N5AsyncReader::exists(self, &path)
    }

    fn read_block<T>(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: GridCoord,
    ) -> Box<dyn Future<Item = Option<VecDataBlock<T>>, Error = Error>>
            where VecDataBlock<T>: DataBlock<T>,
                  T: ReflectedType + 'static {

        Box::new(N5AsyncEtagReader::read_block_with_etag(
                self, path_name, data_attrs, grid_position)
            .map(|maybe_block| maybe_block.map(|(block, _etag)| block)))
    }

    fn list(&self, _path_name: &str) -> Box<dyn Future<Item = Vec<String>, Error = Error>> {
        // TODO: Not implemented because remote paths are not listable.
        unimplemented!()
    }

    fn list_attributes(
        &self,
        path_name: &str,
    ) -> Box<dyn Future<Item = serde_json::Value, Error = Error>> {

        Box::new(self.get_attributes(path_name))
    }
}

impl N5AsyncEtagReader for N5HTTPFetch {
    fn block_etag(
        &self,
        path_name: &str,
        _data_attrs: &DatasetAttributes,
        grid_position: GridCoord,
    ) -> Box<dyn Future<Item = Option<String>, Error = Error>> {
        let mut request_options = RequestInit::new();
        request_options.method("HEAD");
        request_options.mode(RequestMode::Cors);

        let block_path = self.relative_block_path(path_name, &grid_position);

        let req = Request::new_with_str_and_init(
            &format!("{}/{}", &self.base_path, block_path),
            &request_options).unwrap();

        let req_promise = web_sys::window().unwrap().fetch_with_request(&req);

        let f = JsFuture::from(req_promise)
            .map(|resp_value| {
                assert!(resp_value.is_instance_of::<Response>());
                let resp: Response = resp_value.dyn_into().unwrap();

                if resp.ok() {
                    resp.headers().get("ETag").unwrap_or(None)
                } else {
                    None
                }
            });

        Box::new(map_future_error_rust(f))
    }

    fn read_block_with_etag<T>(
        &self,
        path_name: &str,
        data_attrs: &DatasetAttributes,
        grid_position: GridCoord,
    ) -> Box<dyn Future<Item = Option<(VecDataBlock<T>, Option<String>)>, Error = Error>>
            where VecDataBlock<T>: DataBlock<T>,
                  T: ReflectedType + 'static {

        let da2 = data_attrs.clone();

        let block_path = self.relative_block_path(path_name, &grid_position);

        let f = self.fetch(&block_path).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();

            if resp.ok() {
                let etag: Option<String> = resp.headers().get("ETag").unwrap_or(None);
                let to_return = JsFuture::from(resp.array_buffer().unwrap())
                    .map(move |arrbuff_value| {
                        assert!(arrbuff_value.is_instance_of::<ArrayBuffer>());
                        let typebuff: js_sys::Uint8Array = js_sys::Uint8Array::new(&arrbuff_value);
                        let buff = typebuff.to_vec();

                        Some((<n5::DefaultBlock as n5::DefaultBlockReader<T, &[u8]>>::read_block(
                            &buff,
                            &da2,
                            grid_position).unwrap(),
                            etag))
                    });
                future::Either::A(to_return)
            } else  {
                future::Either::B(future::ok(None))
            }
        });

        Box::new(map_future_error_rust(f))
    }
}
