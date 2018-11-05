# EPRIQ

Erlang Priority Queue (EPRIQ) based on pairing heap algorithm.

## Pairing Heap

Pairing heap is a sorting algorithm with good practical amortized performance. The amortized time per delete-node is O(log n), and the operations find-min(max), and add-node run in O(1) amortized time.

### How to use?

You can use this module as regular Erlang modules or compile and test it by this recipe:
(write these commands on Erlang REPL)
```
c(epriq).
c(epriq_test).
epriq_test:run().
```
It creates a Queue with random priority between 1 and 10 and then prints nodes that added to this queue based on priority.

## Author

* **Mohsen Moqadam** - [BisPhone](https://bisphone.com/en/home)

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details
