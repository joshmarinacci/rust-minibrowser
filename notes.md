# inline layout

all inline layout happens inside the anonymous block.  it grabs the text child
and produces a series of line blocks for it.

to handle proper nested styles we need to handle multiple elements with text child inside of the anonymous block. the algorithm should work roughly like this:


create a line box
extract text from the current child
    if no more text on this child, go to the next child.
calculate style for the text
calculate dimensions for the text
if the text is too long for the line box
    split text
    create text box for the first half
    add text box to line box
    create a new line box
    loop back to start 
else 
    create text box for the text
    add text box to the line box
    loop back to start

this algorithm will work for a series of inline elements. ex:

``` html
<div>
    some text 
    <b>bold</b>
    more
    <i>italic</i>
    more
</div>
```

However, it won't work with nested styles like this:

``` html
<div>
    some text
    <b> bold <i> bold and italic </i> bold</b>
    more
</div>
```

for this case the logic becomes more complicated, and possibly recursive, which I don't like. A way to handle this is to flatten the nested styles by generating a replacement styled dom (just for styling purposes). If drawn as HTML it would look like this:

``` html
<div>
    some text
    <b> bold </b> <b and i> bold and italic </b and i> <b>bold</b>
</div>
```

this would ensure we never have any nesting.

inside anonymous layout it should call a function to flatten it's children before processing them.

