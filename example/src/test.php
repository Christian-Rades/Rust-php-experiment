<?php

class A {
}

class Flag {
	private $name = "foo";
}

class B extends A {
	public $flags;
	public function __construct() {
		$flag = new Flag();
		$this->flags = [get_class($flag) => $flag];
	}
}

var_dump(\hello_world("David"));
