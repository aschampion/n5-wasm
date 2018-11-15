use std::fmt::Write;
use std::str::FromStr;

use wasm_bindgen::JsCast;

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
        grid_position: Vec<i64>
    ) -> Promise {
        N5PromiseReader::read_block(self, path_name, data_attrs, grid_position)
    }

    pub fn list_attributes(&self, path_name: &str) -> Promise {
        N5PromiseReader::list_attributes(self, path_name)
    }
}

impl N5AsyncReader for N5HTTPFetch {
    fn get_version(&self) -> Box<Future<Item = n5::Version, Error = Error>> {
        let to_return = self.get_attributes("").map(|attr| {
            n5::Version::from_str(attr
                    .get(n5::VERSION_ATTRIBUTE_KEY)
                    .unwrap()
                    .as_str().unwrap_or("")
                ).unwrap()
        });

        Box::new(to_return)
    }

    fn get_dataset_attributes(&self, path_name: &str) ->
            Box<Future<Item = n5::DatasetAttributes, Error = Error>> {

        let to_return = self
            .fetch_json(&format!("{}/{}", path_name, ATTRIBUTES_FILE))
            .map(|json| { json.into_serde().unwrap() });

        Box::new(map_future_error_rust(to_return))
    }

    fn exists(&self, path_name: &str) -> Box<Future<Item = bool, Error = Error>> {
        let to_return = self.fetch(path_name).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();

            future::ok(resp.ok())
        });

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
                  T: Clone + 'static {

        let da2 = data_attrs.clone();

        let mut block_path = String::new();
        for coord in &grid_position {
            write!(block_path, "/{}", coord);
        }

        let f = self.fetch(&format!("{}{}", path_name, block_path)).and_then(|resp_value| {
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();

            if resp.ok() {
                let to_return = JsFuture::from(resp.array_buffer().unwrap())
                    .map(move |arrbuff_value| {
                        assert!(arrbuff_value.is_instance_of::<ArrayBuffer>());
                        let typebuff: js_sys::Uint8Array = js_sys::Uint8Array::new(&arrbuff_value);

                        // TODO: tedious buffer copy.
                        // See: https://github.com/rustwasm/wasm-bindgen/issues/811
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

    fn list(&self, _path_name: &str) -> Box<Future<Item = Vec<String>, Error = Error>> {
        // TODO: Not implemented because remote paths are not listable.
        unimplemented!()
    }

    fn list_attributes(
        &self,
        path_name: &str,
    ) -> Box<Future<Item = serde_json::Value, Error = Error>> {

        self.get_attributes(path_name)
    }
}
