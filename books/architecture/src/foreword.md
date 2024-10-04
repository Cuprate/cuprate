# Foreword
Monero[^1] is a large software project, coming in at 329k lines of C++, C, headers, and make files.[^2] It is directly responsible for 2.6 billion dollars worth of value.[^3] It has had over 400 contributors, more if counting unnamed contributions.[^4] It has over 10,000 node operators and a large active userbase.[^5]

The project wasn't always this big, but somewhere in the midst of contributors coming and going, various features being added, bugs being fixed, and celebrated cryptography being implemented - there was an aspect that was lost by the project that it could not easily gain again: **maintainability**.

Within large and complicated software projects, there is an important transfer of knowledge that must occur for long-term survival. Much like an organism that must eventually pass the torch onto the next generation, projects must do the same for future contributors.

However, newcomers often lack experience, past contributors might not be around, and current maintainers may be too busy. For whatever reason, this transfer of knowledge is not always smooth.

There is a solution to this problem: **documentation**.

The activity of writing the what, where, why, and how of the solutions to technical problems can be done in an author's lonesome.

The activity of reading these ideas can be done by future readers at any time without permission.

These readers may be new prospective contributors, it may be the current maintainers, it may be researchers, it may be users of various scale.  Whoever it may be, documentation acts as the link between the past and present; a bottle of wisdom thrown into the river of time for future participants to open.

This book is the manifestation of this will, for Cuprate[^6], an alternative Monero node. It documents Cuprate's implementation from head-to-toe such that in the case of a contributor's untimely disappearance, the project can continue.

People come and go, documentation is forever.

— hinto-janai

---

[^1]: [`monero-project/monero`](https://github.com/monero-project/monero)

[^2]: `git ls-files | grep "\.cpp$\|\.h$\|\.c$\|CMake" | xargs cat | wc -l` on [`cc73fe7`](https://github.com/monero-project/monero/tree/cc73fe71162d564ffda8e549b79a350bca53c454)

[^3]: 2024-05-24: $143.55 USD * 18,151,608 XMR = $2,605,663,258

[^4]: `git log --all --pretty="%an" | sort -u | wc -l` on [`cc73fe7`](https://github.com/monero-project/monero/tree/cc73fe71162d564ffda8e549b79a350bca53c454)

[^5]: <https://monero.fail/map>

[^6]: <https://github.com/Cuprate/cuprate>