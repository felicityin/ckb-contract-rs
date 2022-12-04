# Build
```
cargo build
```

# Run
```
cargo run
```

# Results

1. Build a multisig address with multiple participants and absolute since value

    ```
    multisig address: ckt1qpw9q60tppt7l3j7r09qcp7lxnp3vcanvgha8pmvsa3jplykxn32sqgzvufpl4t0yks2uwyzx82cdlscmglxl0svza0k8
    ```

2. Deploy a contract binary with multisig address and type id type script
    The first cell of outputs: https://pudge.explorer.nervos.org/transaction/0x43004199d66fff32b3d175ce5959e02f413a73f16abeba0e140deaa6c07c1abb

3. Update above contract binary with new multisig address (keep the type script)
   The first cell of outputs: https://pudge.explorer.nervos.org/transaction/0x8c27bfd8b3e95b9a76e1544043ca0ec6851c8d0ab6bdbc1df65a902d75c8ed85
