# flora

A Windows 10/11 Widget Daemon

`flora` provides a base for you to build any widget you want using HTML/CSS/JavaScript. `flora` does
_not_ provide any widgets for you, and it's up to you to either find widgets online or write your
own. This repo has some examples to get you started if you decide to make your own.

`flora` is essentially a browser that displays your widgets. In fact, the window has an embedded
webview that uses Edge (Chromium) to display what is essentially webpages.

> [!WARNING]
>
> `flora` is still rough around the edges. Though it _does_ work, it still needs some convenience
> features for a better user experience. Expect some issues to popup if you decide to use it.

## Getting Started

### Installing

`flora` is still in an early state, and as such must be compiled from source. To compile, you will
need `rust`, you can find details about installing rust here: https://www.rust-lang.org/tools/install

After cloning the repository, simply run:

```
cargo build
```

Or, if you want a release version:

```
cargo build --release
```

The compiled binary will be in the `target/` directory.

### Running

If you want to run `flora` directly:

```
cargo run
```

Otherwise, after compiling, you can run the binary directly (located in the `target/` directory).

---

You will notice that the command will fail saying you need to specify a config. You can do so by
running one of the following commands:

Cargo:

```
cargo run -- --config-path <PATH>
```

Or, if running the binary directly:

```
flora --config--path <PATH>
```

You can find configuration details below.

### Configuration

`flora` is configured using a `<config_name>.flora` file. This can be located anywhere and named
anything as long as it has the `.flora` extension.

The config file uses lisp-like syntax (S-expressions), and the configuration options are quite
straightforward. A full list of options is specified in the [docs](./docs/).

A minimum configuration is the following:

```lisp
((content "<div>hi<div>"))
```

Of course, this does not do anything particularly useful, but shows how easily you can get started
with a widget. However, unless you have a very trivial widget, it's better to separate the actual
content of the widget into a separate file. This can be done as follows:

```lisp
((content "file:///absolute/path/to/file")
(content-url))
```

Then all you need to do is ensure the file is available at the path. Additionally, if you really
wanted to, you could even set the content to any URL and the webpage will be rendered.

## Some Remarks

Though a standalone project, this repo is also a bit of exploratory work for
[winbar](https://github.com/sulaxan/winbar) (a Windows 10/11 status bar). `winbar` may eventually
supersede this project, however, I believe there is value in keeping this project separate, and as
such, `flora` will continue to be maintained for the foreseeable future.

## Credits

- The [webview2-rs](https://github.com/wravery/webview2-rs/) crate was a big help in creating this
  project; it does a lot of the heavylifting to make this project possible.
