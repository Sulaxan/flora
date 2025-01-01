# Configuring `flora`

`flora` can be configured using a flora configuration file. This file uses lisp-like (S-expression)
syntax.

The configuration file can be located anywhere and be named anything as long as it has the `.flora`
extension.

## Configuration File

A simple configuration file looks like the following:

```lisp
((content "<div>A flora widget!</div>"))
```

All configuration options are put inside a list (specified by the initial set of brackets `()`). As
an example, if we wanted to set another key, we can do the following:

```lisp
((content "<div>A flora widget!</div>")
(dim 1000 20))
```

## Content

This is the most important key you must set in your configuration file. It dictates what you will
see once you run `flora`. The simplest value we can set is the following:

```lisp
((content "<div>A flora widget!</div>"))
```

This can be any HTML content.

---

The main issue with this approach is it's difficult to work with and maintain if we want to do
something more complicated. `flora` provides the ability to pass in a file instead:

```lisp
((content "file:///path/to/file"))
```

Note that the path must be an **absolute path**.

This must be the absolute path to the file. However, if you try using this config now, you will
notice it doesn't quite work. This is because we need to let `flora` know to interpret the content
as a URL using the `content-url` option:

```lisp
((content "file:///path/to/file")
(content-url))
```

This should now work as expected assuming our path is valid.

---

As an extension to the above, `flora` also allows you to specify a URL to a webpage:

```lisp
((content "https://github.com/sulaxan/flora")
(content-url))
```

This will render the page in the widget.

---

As a closing note, `flora` is essentially a web browser under the hood, so if there's a link your
browser can visit and display, chances are `flora` can as well.

## Configuration Options

Below is a list of all options you can specify in the configuration file:

| Option                 | Description                                                                    |
| ---------------------- | ------------------------------------------------------------------------------ |
| (position _x_ _y_)     | Sets the position of the widget window. Must be a valid bound for your screen. |
| (dim _width_ _height_) | Sets the dimensions of the window.                                             |
| (content _string_)     | Sets the content to display. Either HTML or a URL.                             |
| (content-url)          | Indicates that `content` should be treated as a URL.                           |
