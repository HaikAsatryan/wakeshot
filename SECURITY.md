# Security

## Supported versions

Only the latest release gets fixes. A fix ships as a new release, and Wakeshot updates itself to it.

## Report a problem

Please do not open a public issue. Report it privately on GitHub:
[Report a vulnerability](https://github.com/HaikAsatryan/wakeshot/security/advisories/new), also under this repo's
Security and quality tab. The report stays private until we publish an advisory.

Tell me:

- the Wakeshot version and your Windows version;
- what an attacker can do, and what they need first (another user account on the PC, a program running as you, your
  network);
- the steps to show it.

What I most want to hear about: a way to make the updater install something that is not a Wakeshot release; a way for
another user account to change your power settings or Wakeshot's files in `%LocalAppData%\Wakeshot`; private data in
the log or in the file `wakeshot diagnostics` saves, such as prompts, transcript text, full command lines, tokens or
your user name.

## What to expect

Wakeshot is a free project with one maintainer. I aim to answer within 7 days. Once the problem is confirmed, I work on
a fix with you, ship it as a new release, then publish the advisory. It credits you, unless you ask me not to.

Already known, so no need to report it: v1 is not code-signed, so Windows SmartScreen may warn before Setup runs
(README, Install).
