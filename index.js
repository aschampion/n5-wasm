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
      reader.exists("foobar")
        .then(exists => {
          console.log("foobar:" + exists);
        }),
      reader.get_dataset_attributes("volume")
        .then(data_attrs => {
          console.log("volume attributes:" + data_attrs);
          return reader.read_block("volume", data_attrs, [0, 0, 0].map(BigInt));
        })
        .then(block => {
          console.log("block:" + (block == null));
          console.log(block);
        })
    ])
	});