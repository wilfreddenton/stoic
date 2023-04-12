# stoic

is a simple static blog generator created to learn about Rust and its library ecosystem.

## Installation

```
cargo install stoic
```

## Usage

```
stoic new blog
```

generates a new blog in the `blog/` directory with file structure:

```
blog
├── about.md
├── index.md
├── assets
│   ├── script.js
│   └── style.css
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
stoic build blog dist
```

to have `stoic` generate the corresponding static html in the `dist/` directory with file structure:

```
dist/
├── about.html
├── index.html
├── assets
│   ├── script.js
│   └── style.css
└── posts
    ├── 2022-01-26-hello-world.html
    └── index.html
```

```
stoic watch blog dist
```

tells `stoic` to watch the `blog/` for changes and rebuild the blog automatically.

It will run a static web server @ `0.0.0.0:3030`.
After each rebuild your browser should automatically reload.

### Collections

The site created by the `new` command above contains a single collection: `posts`.
The collection is identified by the name of the folder.
All markdown files in this folder are treated as items of this collection.
In the `templates` folder are the `posts.hbs` and `post.hbs` templates.
The `posts.hbs` template is used when generating the index of the collection.
The `post.hbs` template is used when generating each item in the collection.

To create a new collection i.e. `works`:

1. create folder `works`
2. create `templates/works.hbs` template
3. create `templates/work.hbs` template
4. put work markdown files in the `works` folder

### Collection Item Metadata

Markdown items in collection folders should contain a metadata section at the top of the file:

```html
<!--metadata
date = 2023-03-25
shortname = "Foo Bar"
slug = "foo_bar
-->
```

`date` is a date in the `YYYY-MM-DD` format.
The date should exist in all collection items as it is used for sorting.

`shorname` is the label used for the entity in the breadcrumbs.
If it is not provided then the date will be used.

`slug` is a name for the output file.
If you have an input file `foo.md` but want the output file to be `foo_bar.html` instead of the default `foo.html`, set the slug to `foo_bar`.

## Examples

- [unsafe-perform.io](https://unsafe-perform.io/) - [repo](https://github.com/wilfreddenton/unsafe-perform.io)
