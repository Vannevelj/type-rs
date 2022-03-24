Utility to convert your Javascript files to Typescript. It will

* Change the file extension to either .ts or .tsx
* Introduce type annotations where necessary (function parameters, ambiguous locals, catch clauses, etc)
* Rudimentary support for React-specific logic, e.g. by updating classes to `Component<any, any>` and `PureComponent<any, any>`


Limitations:
* JSX is not supported and the tool might introduce some minor issues. These are easily solved though: from what I've seen, `: any` is added to some callbacks that are included in `<>` JSX tags
* type-rs adds the type annotations but you'll still have to define your custom types, import `@types/` dependencies, etc
* Flow is not supported. If you want to convert flow code to Typescript, I recommend [`flow-to-ts`](https://github.com/Khan/flow-to-ts). These files break rslint too much so if a Flow-enabled file is encountered, we skip it altogether.

Design choices:
* type-rs uses [rslint](https://github.com/rslint/rslint) under the hood. rslint has the disadvantage of not supporting JSX. I considered using swc instead but decided against it because swc [does not preserve whitespace](https://github.com/swc-project/swc/discussions/4079#discussioncomment-2426512). The inconvenience of some bugs in JSX code seemed less severe than having the code re-formatted. In general I also found rslint to have a much nicer API to work with.
* We spawn a thread for each file that gets converted. This is done for two reasons: 

1. It's faster
2. It isolates each file's panics. Sometimes rslint errors out, presumably because of JSX. If it happens there's no recovery anyway so this allows me not to care about it at all.