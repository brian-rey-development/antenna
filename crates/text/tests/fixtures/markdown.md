# Field Notes

The first paragraph has *emphasis*, **strong text** and ~~struck text~~ in one line. It also has a [link to the guide](https://example.com/guide) and an autolink <https://example.com/raw> that Antenna drops.
A soft break joins this line to the line before it, and a hard break  
starts a new line inside the same paragraph.

![A diagram of the tower](tower.png)

Use `cargo run` to start the tool. The text `3.14 is a number` stays in the speech.

## Lists and quotes

- First item of the list.
- Second item with a nested list:
  - Nested item one.
  - Nested item two.
- Third item of the list.

1. Open the door.
2. Climb the stair.

> A block quote has its own paragraph. It can hold more than one sentence.
>
> A second quoted paragraph follows.

---

## Code and markup

```rust
fn main() {
    println!("Antenna does not read code blocks.");
}
```

    An indented block is code too.

<div class="note">
This HTML block is silent.
</div>

Inline <b>HTML</b> tags are silent, but the text between them stays. Entities such as &amp; and &copy; and escapes such as \* stay readable.

| Name | Value |
|------|-------|
| Height | 31.5 metres |
| Period | 12.5 seconds |

The last paragraph closes the notes, and it is short.
