defmodule RustlerBtleplug.NativeTest do
  @moduledoc false
  use ExUnit.Case, async: true
  alias RustlerBtleplug.Native

  test "Test string" do
    test_string = "test string"
    assert Native.test_string(test_string) == test_string, "Expected #{inspect(test_string)}"
  end

  test "Test add" do
    assert Native.add(5, 5) == 10, "Expected 10"
  end

  test "Test map" do
    map = Native.get_map()
    assert is_map(map), "Expected map"
  end

end
