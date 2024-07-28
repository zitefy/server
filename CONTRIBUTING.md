# Contributing

First of all, a huge thank you for looking to contribute to this repository! This project was concieved and built at [FOSS Hack 4.0](https://fossunited.org/fosshack/2024), a hackathon for building open-source software. As such, everything from the architecture of this server to the code has been *hacked up* in a couple of hours. That is to say, this is messed up lol. There's tons of stuff to improve.

## Improvements
These are some of the stuff that we'd like to improve upon

### Reduce memory footprint
Any changes to reduce the overall memory footprint of the application are welcome. We're sure there are lots of `String` variables instead of string literals waiting for you to get started with. 😂 Just for context, there are 36 `.clone()` calls throughout the codebase right now! That's 36 deep copy operations waiting to be removed.

Any changes improving performance in any other way are appreciated as well. Eliminating JavaScript is welcome. More on that below.

### Complete overhaul?
Sure, why not! As said earlier, this was hacked up and we're not sure this is the best approach to build this server. Right now, it works and that's all we know! If you find a way to overhaul it and make it better, please do so and open a PR!

> Note: If your PR changes any of the endpoint behaviors or names, please coordinate with us by [messaging me on telegram](https://t.me/vishalds/).

### Eliminating JavaScript
The screenshot script can maybe be replaced, but we couldn't find an effective solution for that right now. The most effective workaround for this would be to switch the previews altogether to use an `<iframe />` in the site. We did try this during the hackathon but couldn't quite get it to work. However, that is outside the scope of this repository. We'd still need screenshots for the previews, so any attempts to move this to rust are appreciated.

As for the HTML string builder script, there currently seem to be no effective solutions to parse HTML reliably with rust alone. We briefly tried [scraper](https://docs.rs/scraper/latest/scraper/) & [select](https://docs.rs/select/latest/select/) but both of these ended up not having solutions to edit the HTML content. They only allow us to parse a string into different elements. So, we resorted to use JS and native DOM for this. Any attempts to move this to rust are appreciated as well.

### Containerization
Usually, servers like these are containerized on deployment. However, since we're not familiar with setting up containers frequently, we couldn't risk doing this at the hackathon.

So, any attempts to containerize this are welcome. A good solution seems to be [docker](https://docker.com/). We're open to anything though.

### Improve code quality
If you can find places to refactor, improve code quality, or do any such thing, feel free to open a PR.

## How to contribute?
Simple, just open a PR. Let the checks run. If the build check is successful and your description conveys the intent, then we'll probably merge it.

You can also open an issue if you spot one.

## Merging right now...
Right now, the evaluation for FOSS Hack is still ongoing. As such, we won't be merging any PR's until August 20, 2024. But feel free to open one, we'll merge once the evaluation finishes.