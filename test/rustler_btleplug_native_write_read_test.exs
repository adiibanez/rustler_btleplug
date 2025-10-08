defmodule RustlerBtleplug.NativeWriteReadTest do
  use ExUnit.Case, async: false
  alias RustlerBtleplug.Native

  # These should be set to match actual test device characteristics
  @test_peripheral_name "toio"
  @test_write_characteristic_uuid "10b20102-5b3b-4571-9508-cf3efcd7bbae"
  @test_read_characteristic_uuid "10b20101-5b3b-4571-9508-cf3efcd7bbae"
  @test_timeout 10_000

  describe "write_characteristic/4" do
    @tag :integration
    test "writes data to a writable characteristic" do
      central_resource =
        Native.create_central()
        |> Native.start_scan()

      assert is_reference(central_resource)
      assert_receive {:btleplug_scan_started, _msg}, 2000

      Process.sleep(1000)

      receive do
        {:btleplug_peripheral_discovered, peripheral_id, _props} ->
          peripheral_resource =
            central_resource
            |> Native.find_peripheral_by_name(@test_peripheral_name)
            |> Native.connect()

          assert is_reference(peripheral_resource)

          assert_receive {:btleplug_peripheral_connected, _msg},
                         @test_timeout,
                         "No :btleplug_peripheral_connected received"

          assert_receive {:btleplug_peripheral_updated, _msg, properties},
                         @test_timeout,
                         "No :btleplug_peripheral_updated received"

          assert is_map(properties)

          # Write motor stop command: [0x01, 0x01, 0x00, 0x00, 0x02, 0x00, 0x00]
          write_result =
            Native.write_characteristic(
              peripheral_resource,
              @test_write_characteristic_uuid,
              [0x01, 0x01, 0x00, 0x00, 0x02, 0x00, 0x00],
              @test_timeout
            )

          assert is_reference(write_result)

          # Cleanup
          Native.disconnect(peripheral_resource)

      after
        @test_timeout * 2 -> flunk("Did not receive :btleplug_peripheral_discovered message")
      end
    end

    @tag :integration
    test "handles write to non-writable characteristic gracefully" do
      central_resource =
        Native.create_central()
        |> Native.start_scan()

      assert is_reference(central_resource)
      assert_receive {:btleplug_scan_started, _msg}, 2000

      Process.sleep(1000)

      receive do
        {:btleplug_peripheral_discovered, _peripheral_id, _props} ->
          peripheral_resource =
            central_resource
            |> Native.find_peripheral_by_name(@test_peripheral_name)
            |> Native.connect()

          assert is_reference(peripheral_resource)

          assert_receive {:btleplug_peripheral_connected, _msg},
                         @test_timeout,
                         "No :btleplug_peripheral_connected received"

          # Attempt to write to a read-only characteristic (should log warning but not crash)
          write_result =
            Native.write_characteristic(
              peripheral_resource,
              @test_read_characteristic_uuid,
              [0x00],
              @test_timeout
            )

          assert is_reference(write_result)

          # Cleanup
          Native.disconnect(peripheral_resource)

      after
        @test_timeout * 2 -> flunk("Did not receive :btleplug_peripheral_discovered message")
      end
    end
  end

  describe "read_characteristic/3" do
    @tag :integration
    test "reads data from a readable characteristic" do
      central_resource =
        Native.create_central()
        |> Native.start_scan()

      assert is_reference(central_resource)
      assert_receive {:btleplug_scan_started, _msg}, 2000

      Process.sleep(1000)

      receive do
        {:btleplug_peripheral_discovered, _peripheral_id, _props} ->
          peripheral_resource =
            central_resource
            |> Native.find_peripheral_by_name(@test_peripheral_name)
            |> Native.connect()

          assert is_reference(peripheral_resource)

          assert_receive {:btleplug_peripheral_connected, _msg},
                         @test_timeout,
                         "No :btleplug_peripheral_connected received"

          assert_receive {:btleplug_peripheral_updated, _msg, properties},
                         @test_timeout,
                         "No :btleplug_peripheral_updated received"

          assert is_map(properties)

          # Read from position ID characteristic
          read_result =
            Native.read_characteristic(
              peripheral_resource,
              @test_read_characteristic_uuid,
              @test_timeout
            )

          assert is_reference(read_result)

          # Should receive read data as message
          assert_receive {:btleplug_characteristic_read, uuid, data},
                         @test_timeout,
                         "No :btleplug_characteristic_read received"

          assert is_binary(uuid)
          assert is_binary(data)

          # Cleanup
          Native.disconnect(peripheral_resource)

      after
        @test_timeout * 2 -> flunk("Did not receive :btleplug_peripheral_discovered message")
      end
    end

    @tag :integration
    test "handles read from non-readable characteristic gracefully" do
      central_resource =
        Native.create_central()
        |> Native.start_scan()

      assert is_reference(central_resource)
      assert_receive {:btleplug_scan_started, _msg}, 2000

      Process.sleep(1000)

      receive do
        {:btleplug_peripheral_discovered, _peripheral_id, _props} ->
          peripheral_resource =
            central_resource
            |> Native.find_peripheral_by_name(@test_peripheral_name)
            |> Native.connect()

          assert is_reference(peripheral_resource)

          assert_receive {:btleplug_peripheral_connected, _msg},
                         @test_timeout,
                         "No :btleplug_peripheral_connected received"

          # Attempt to read from a write-only characteristic (should log warning but not crash)
          read_result =
            Native.read_characteristic(
              peripheral_resource,
              @test_write_characteristic_uuid,
              @test_timeout
            )

          assert is_reference(read_result)

          # Cleanup
          Native.disconnect(peripheral_resource)

      after
        @test_timeout * 2 -> flunk("Did not receive :btleplug_peripheral_discovered message")
      end
    end
  end
end
