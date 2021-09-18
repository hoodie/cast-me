# Cast Me

tiny demo of a pure rust broker to establish a simple peerconnection for minimal screensharing

for fun

## What to do with it

```
# prepare the front end app
yarn --cwd app build

# run the everything else
RUST_LOG=info,cast_me=trace cargo run
```

Now open the https://0.0.0.0:3030 twice and enter the code from one instance into the input of the other.
you can now chat and screenshare!
