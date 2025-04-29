#include <iostream>

extern "C" {

  void foo() { std::cout << "Hello World" << '\n'; }
}


