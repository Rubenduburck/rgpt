## WIP

Overcomplicated POS but it does what it needs to do.
Bit generic because I had plans of integrating multiple providers but I'll see when I have time for that.

### Supports
- [x] Anthropic
- [ ] Everything else

### Usage
```bash
$ rgpt-cli --mode <mode> <input>
```

E.g.

```bash
$ rgpt-cli --mode bash "How can I list the files in this directory?"
  exec ls
> exit
```
```bash
$ rgpt-cli --mode bash "Can you give me several commands to list files?"
  exec ls -l
  exec ls -la
  exec ls -R
  exec ls -lh
  exec ls -t
  exec ls -S
> exit
```

## TODO
- [ ] lots
