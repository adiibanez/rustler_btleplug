defmodule RustlerBtleplug.Client do
  require Logger

  def add(a, b) do
    case RustlerBtleplug.Native.add(a, b) do
      {:ok, result} ->
        Logger.info("Rustler add #{inspect(result)}")

      {:error, err} ->
        {:error, err}

      # res -> {:ok, NifIo.FileHandle.wrap_resource(res)}
      number ->
        {:ok, number}
    end
  end

  def get_map() do
    case RustlerBtleplug.Native.get_map() do
      {:ok, result} ->
        Logger.info("Rustler add #{inspect(result)}")

      {:error, err} ->
        {:error, err}

      # res -> {:ok, NifIo.FileHandle.wrap_resource(res)}

      number ->
        {:ok, number}
    end
  end

  @spec scan() :: :ok | {:error, any()} | {:ok, any()}
  def scan() do
    case RustlerBtleplug.Native.scan() do
      {:ok, result} ->
        Logger.info("Rustler scan #{inspect(result)}")

      {:error, err} ->
        {:error, err}

      # res -> {:ok, NifIo.FileHandle.wrap_resource(res)}

      result ->
        Logger.info("Rustler scan #{inspect(result)}")
        {:ok, result}
    end
  end

  def connect(uuid) do
    case RustlerBtleplug.Native.connect(uuid) do
      {:ok, result} ->
        Logger.info("Rustler connect #{inspect(uuid)} #{inspect(result)}")

      {:error, err} ->
        {:error, err}

      result ->
        Logger.info("Rustler connect #{inspect(result)}")
        {:ok, result}
    end
  end

  def init() do
    case RustlerBtleplug.Native.init() do
      {:ok, result} ->
        Logger.info("Rustler init #{inspect(result)}")

      # res -> {:ok, NifIo.FileHandle.wrap_resource(res)}
      {:error, err} ->
        {:error, err}

      number ->
        {:ok, number}
    end
  end
end
