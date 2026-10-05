# External author ergonomics correction

The complete a7b48ff producer passed its first seven gates and 28 standard actual-host cases.
Its independent application test then tried to open a dialog that the inline renderer had replaced.
A second obsolete assertion expected revision text without the Resource kind.

The correction changes the maintained example test, not production code or the SDK.
The test locates the visible inline control and performs the ordinary save operation.
It verifies canonical TICKET-AUTHOR2, revision two, and a separate ordinary Task mutation.
It retains authentication, deadlines, both databases, and both browser engines. No case is skipped.

| Verification | Result | Evidence |
| --- | --- | --- |
| Original dialog selector | Expected failure | [RED](evidence/rom-0.0.3/external-author-ergonomics/red.log) |
| Inline control with old revision selector | Expected failure | [Second RED](evidence/rom-0.0.3/external-author-ergonomics/inline.log) |
| Corrected browser matrix | Four passes; no skips or retries | [GREEN](evidence/rom-0.0.3/external-author-ergonomics/green.log) |
| Fresh extracted public SDK application | Offline install, typecheck, build, and four browser cases passed | [Result](evidence/rom-0.0.3/external-author-ergonomics/extracted-result.json) |
| Full local verifier | Passed | [Log](evidence/rom-0.0.3/external-author-ergonomics/full-check.log) |

The [source record](evidence/rom-0.0.3/external-author-ergonomics/source-facts.json) identifies the executed test and immutable native binary.
The test SHA256 is 2c4a5ff7b0107494bdae6551cf40f4193f866a555fcf69abf262a6e1ba9ea55c.
These checks precede the corrected clean-source producer. They do not establish a complete release or preview deployment.
Failed producer stages remain preserved outside the accepted evidence set.
