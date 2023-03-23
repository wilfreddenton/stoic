# patina

is a simple static blog generator created to learn about Rust and its library ecosystem.

## Installation

```
cargo install patina
```

## Usage

```
patina new blog
```

generates a new blog in the `blog/` directory with file structure:

```
blog
├── assets
│   ├── script.js
│   └── style.css
├── index.md
├── pages
│   └── about.md
├── posts
│   └── 2022-01-26-hello-world.md
└── templates
    ├── base.hbs
    ├── index.hbs
    ├── nav.hbs
    ├── page.hbs
    ├── post.hbs
    └── posts.hbs
```

You can run:

```
patina build blog dist
```

to have `patina` generate the corresponding static html in the `dist/` directory with file structure:

```
dist/
├── about.html
├── assets
│   ├── script.js
│   └── style.css
├── index.html
└── posts
    ├── 2022-01-26-hello-world.html
    └── index.html
```

```
patina watch blog dist
```

tells patina to watch the `blog/` for changes and rebuild the blog automatically.

## Examples

- [unsafe-perform.io](https://unsafe-perform.io/) - [repo](https://github.com/wilfreddenton/unsafe-perform.io)
