defmodule RustlerBtleplugTest do
  use ExUnit.Case
  # doctest RustlerBtleplug

  # test "add 2 numbers" do
  #   case RustlerBtleplug.Client.add(5, 5) do
  #     {:ok, number} ->
  #       IO.puts("ok: #{number}")
  #       assert number == 25

  #     result ->
  #       IO.puts("not sure: #{inspect(result)}")
  #       assert true
  #   end

  #   # IO.puts("Number: #{number}")
  # end

  # test "get map " do
  #   case RustlerBtleplug.Client.get_map() do
  #     {:ok, map} ->
  #       IO.puts("ok: #{inspect(map)}")
  #       assert is_map(map)

  #     result ->
  #       IO.puts("not sure: #{inspect(result)}")
  #   end

  #   # IO.puts("Number: #{number}")
  # end

  test "scan" do
    case RustlerBtleplug.Client.init() do
      {:ok, pid} when is_pid(pid) ->
        IO.inspect("PID: #{inspect(pid)}")
        assert true

      {:ok, result} ->
        IO.puts("ok 2: #{inspect(result)}")

        case RustlerBtleplug.Client.scan() do
          {:ok, map} ->
            IO.puts("ok 3: #{inspect(map)}")
            assert is_map(map)

          result ->
            IO.puts("not sure: #{inspect(result)}")
        end

      {:error, :invalid_options} ->
        IO.puts("Invalid options")

      {:error, error} ->
        IO.puts("error: #{inspect(error)}")

      :ok ->
        IO.puts("simple :ok")

      _ ->
        IO.puts("not sure 2")
    end

    IO.puts("test")

    Process.sleep(2000)

    messages = :erlang.process_info(self(), :messages)
    IO.inspect(messages, label: "messages")

    assert_receive {:btleplug_got_central, _}
     #no_adapters_found,
    assert_receive {:btleplug_device_discovered, _}
  end
end
