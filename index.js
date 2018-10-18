const js = import("./pkg");

js
  .then(js => {
    return js.N5HTTPFetch.open("http://localhost:8090")
  })
  .then(reader => {
    return Promise.all([
      reader.get_version()
        .then(version => {
          console.log(version.to_string());
        }),

      reader.exists("volume")
        .then(exists => {
          console.log("volume:" + exists);
        }),

      reader.dataset_exists("volume")
        .then(exists => {
          console.log("volume is dataset:" + exists);
        }),

      reader.exists("foobar")
        .then(exists => {
          console.log("foobar:" + exists);
        }),

      reader.list_attributes("volume")
        .then(attrs => {
          console.log(attrs);
        }),

      reader.get_dataset_attributes("volume")
        .then(data_attrs => {
          console.log("volume attributes:" + data_attrs.get_dimensions());
          return reader.read_block("volume", data_attrs, [0, 0, 0].map(BigInt));
        })
        .then(block => {
          console.log("block:" + (block == null));
          console.log(block);
          console.log(block.get_size());
          console.log(block.get_grid_position());
          console.log(block.get_data());
          console.log(block.get_num_elements());
        })
    ])
	});