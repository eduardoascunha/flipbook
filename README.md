# flipbook

Aplicação de terminal para registar compras e vendas, com os dados numa base de dados SQLite local.

## Compilar e usar

```sh
cargo build --release
./target/release/flipbook
```

A base de dados fica em `~/Library/Application Support/flipbook/flipbook.db`; `--db <caminho>` usa outro ficheiro.

| Tecla | Ação |
|---|---|
| `Tab` / `←` / `→` | muda de lista |
| `↑` / `↓` | seleciona uma linha (`PgUp` / `PgDn` saltam 10) |
| `s` | muda a ordem (mais recentes, mais caras, mais baratas) |
| `a` / `e` | adiciona / edita (`Tab` muda de campo, `Enter` guarda, `Esc` cancela) |
| `d` | apaga a linha selecionada (`s` confirma, `n` cancela) |
| `q` / `Esc` | sai |

## Exportar e importar

```sh
flipbook export <ficheiro.json>
flipbook restore <ficheiro.json>
```

`restore` substitui os dados atuais e pede confirmação se a base de dados já tiver dados (`--yes` salta o pedido).

----

# flipbook

Terminal app to track purchases and sales, with the data in a local SQLite database.

## Build and use

```sh
cargo build --release
./target/release/flipbook
```

The database lives in `~/Library/Application Support/flipbook/flipbook.db`; `--db <path>` uses another file.

| Key | Action |
|---|---|
| `Tab` / `←` / `→` | switch list |
| `↑` / `↓` | select a row (`PgUp` / `PgDn` jump 10) |
| `s` | change the sort (newest, most expensive, cheapest) |
| `a` / `e` | add / edit (`Tab` next field, `Enter` save, `Esc` cancel) |
| `d` | delete the selected row (`s` confirms, `n` cancels) |
| `q` / `Esc` | quit |

## Export and import

```sh
flipbook export <file.json>
flipbook restore <file.json>
```

`restore` replaces the current data and asks for confirmation if the database already has data (`--yes` skips the prompt).
