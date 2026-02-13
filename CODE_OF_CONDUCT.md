# Code of Conduct — Helix OS

## Our Pledge

We, as members, contributors, and maintainers of the Helix OS project, pledge to make
participation in our community a harassment-free experience for everyone, regardless of age,
body size, visible or invisible disability, ethnicity, sex characteristics, gender identity
and expression, level of experience, education, socioeconomic status, nationality, personal
appearance, race, caste, color, religion, or sexual identity and orientation.

We pledge to act and interact in ways that contribute to an open, welcoming, diverse,
inclusive, and healthy community.

---

## Our Standards

### ✅ Positive Behavior

Examples of behavior that contributes to a positive environment:

- **Being respectful** — You may disagree on a technical decision, but always
  address the argument, not the person.
- **Giving and receiving constructive feedback** — Code review is a learning
  opportunity for both sides. Explain *why* something should change, not just that
  it should.
- **Accepting responsibility** — If you introduce a regression, own it, fix it,
  and help write the test that prevents it from happening again.
- **Focusing on what is best for the project** — Kernel code runs on bare metal
  with no safety net. Technical correctness and code quality come first.
- **Showing empathy and kindness** — Everyone was new once. Help newcomers find
  their footing in the codebase.
- **Being patient with the review process** — Kernel patches often require
  multiple rounds of review. This is normal and expected.

### ❌ Unacceptable Behavior

Examples of behavior that will not be tolerated:

- Sexualized language, imagery, or unwelcome sexual attention of any kind
- Trolling, insulting or derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information (physical or email address) without
  their explicit permission
- Sustained disruption of discussions, reviews, or community channels
- Dismissing or attacking someone for asking questions or being a newcomer
- Any other conduct that could reasonably be considered inappropriate in a
  professional setting

---

## Scope

This Code of Conduct applies in all community spaces, including but not limited to:

| Space | Examples |
|:------|:---------|
| **Code** | Pull requests, code reviews, issues, commits |
| **Communication** | GitHub Discussions, Discord, IRC, mailing lists |
| **Events** | Meetups, conferences, online calls |
| **Public representation** | Using the project's name or logo in public contexts |

It also applies when an individual is officially representing the community in
public spaces — for example, using an official project email address, posting
via an official social media account, or acting as a designated representative
at an event.

---

## Enforcement

### Reporting

Instances of abusive, harassing, or otherwise unacceptable behavior may be
reported to the project maintainers at:

📧 **conduct@helixos.org**

All reports will be reviewed and investigated promptly and fairly. The project
team is obligated to maintain confidentiality with regard to the reporter of
an incident. You will not face retaliation for making a report in good faith.

### Enforcement Guidelines

Project maintainers will follow these guidelines in determining the
consequences for any action they deem in violation of this Code of Conduct:

#### 1. Correction

**Community Impact:** Use of inappropriate language or other behavior deemed
unprofessional or unwelcome.

**Consequence:** A private, written warning from project maintainers, providing
clarity around the nature of the violation and an explanation of why the
behavior was inappropriate. A public apology may be requested.

#### 2. Warning

**Community Impact:** A violation through a single incident or series of
actions.

**Consequence:** A warning with consequences for continued behavior. No
interaction with the people involved, including unsolicited interaction with
those enforcing the Code of Conduct, for a specified period of time. This
includes avoiding interactions in community spaces as well as external channels
like social media. Violating these terms may lead to a temporary or permanent
ban.

#### 3. Temporary Ban

**Community Impact:** A serious violation of community standards, including
sustained inappropriate behavior.

**Consequence:** A temporary ban from any sort of interaction or public
communication with the community for a specified period of time. No public or
private interaction with the people involved, including unsolicited interaction
with those enforcing the Code of Conduct, is allowed during this period.
Violating these terms may lead to a permanent ban.

#### 4. Permanent Ban

**Community Impact:** Demonstrating a pattern of violation of community
standards, including sustained inappropriate behavior, harassment of an
individual, or aggression toward or disparagement of groups of people.

**Consequence:** A permanent ban from any sort of public interaction within the
community.

---

## Kernel-Specific Guidelines

Because Helix OS is an operating system kernel, we have additional expectations
that go beyond the standard Code of Conduct:

### Technical Integrity

- **Never submit code you know to be unsafe without clearly documenting why.**
  Every `unsafe` block must carry a `// SAFETY:` comment. Hiding unsoundness is
  a trust violation.
- **Do not merge your own patches** unless you are the sole maintainer of a
  subsystem and have waited a reasonable period for review.
- **Security disclosures follow the [Security Policy](SECURITY.md).** Do not
  publicly disclose vulnerabilities before coordinated disclosure.

### Collaboration

- **Credit contributions fairly.** If your patch is based on someone else's
  work or idea, acknowledge it in the commit message or PR description.
- **Respect module ownership.** Each subsystem has maintainers listed in the
  documentation. Coordinate with them before making large-scale changes to
  their code.
- **Write tests for what you ship.** Untested kernel code is a liability for
  everyone who runs Helix.

### Communication

- **English is the project's primary language** for code, documentation, and
  reviews. Contributions in other languages are welcome in Discussions and
  community channels.
- **Keep discussions technical.** GitHub Issues and PRs are for engineering
  work, not politics or off-topic debates.

---

## Attribution

This Code of Conduct is adapted from the
[Contributor Covenant](https://www.contributor-covenant.org/), version 2.1,
available at
[https://www.contributor-covenant.org/version/2/1/code_of_conduct.html](https://www.contributor-covenant.org/version/2/1/code_of_conduct.html).

Enforcement guidelines were inspired by
[Mozilla's code of conduct enforcement ladder](https://github.com/mozilla/diversity).

---

*Last updated: February 2026*
