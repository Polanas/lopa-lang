```rust
//A local-scoped immutable variable
let a: int = 20;
//A mutable variable
var b = 21;

//Still an int
let a_implicint = 20;

enum Idk {
  First(bool),
  Second(bool),
}

let idk = Idk::First(true);
match idk {
    First(value) if value => {

    },
    Second(value) => {

    },
    _ => {},
}

//Error: cannot cast string to int
a = "no";

//A global variable, always mutable
global g = 1.5;

if a > 10 {

} else if a == 20 && a != 0 || a == 15 {

} else {
     
}

//Everything is an expression;
let b = if a > 5 { 0 } else { 15 };

loop {
    if true {
        continue;
    }
    break;
}

while false {}

for i = 1, 20, 2 {
    print(i);
}

let a: int, b: float, c = 10, 15.0, "hello";

//type is int?
let _ = if true {
  let a  = 20; //evaluates to nil
} else {
  20 //evaluates to int
}


let value: int? = 20;
//error: cannot convert int? to int
let not_nil: int = value;

if value {
    //value is int
}

if let idk = value {
    //idk is int
}

//local function definition
let add = fn(var a: int: b: int) -> int {
    a += 1;
    //erorr
    b += 1;
    a + b
};

//a closure
let add_closure = |a: int, b: int| a = b;

//r = 3
let r = add_closure(1,2);

//scope, shadowing
{
  let value = "hi";
  let value = 20;
}

let call_fn = fn(func: fn(int, int) -> int, a: int, b: int) -> int {
    func(a,b)
}

//functions are first class objects
call_fn(add, 1,2);

//A collection of fields
struct Point {
    x: float,
    y: float, //last , is optional
}

let p = Point {
    x: 1, y: 2
};

struct WrapperStruct(int);

/*Important note: lua is just the backend. All it needs to do is accomplish the goal.
There's no need to create a Point function, for example.
something like 
 local p = {
    x = 1, y = 2, __type = "Point"
 }
is enough
*/

impl Point {
/*
  Now this is where it gets interesting.
  At the end of the day, all functions/methods from all impl blocks will need to be collected together into one metatable.
*/

  pub fn new(x: float, y: float) -> Self {
    return Self(x,y)
  }
}

let point = Point::new(1.0,2.0);
match point {
    Point { x: > 2, y: < 1 } => {

    },
    _ => {},
  }

let str1 = "hello";
let str2 = 'hi' + "hey";
```
