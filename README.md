# Usage

Count lines of all files in current directory recursively
```shell
line-count .
```

Count lines of all files in current directory non-recursively
```shell
line-count . --no-recurse
```

Count lines of all files in group of directories recursively
```shell
line-count /dir/a /dir/b /dir/c
```

Filter files to count. This regex only match file name, not full directory
```shell
line-count . -r "(\.(rs|md|toml))$"
```

Filter files to count with negative filter. Matched files are skipped
```shell
line-count . -r "(\.(rs|md|toml))$" --regex-not
```
