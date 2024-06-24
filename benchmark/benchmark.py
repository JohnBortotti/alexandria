import concurrent.futures
import time
import io
import random
import string
import pycurl
from multiprocessing import Pool

DB_URLS = ["http://192.30.0.101:5000", "http://192.30.0.102:5000", "http://192.30.0.103:5000"]
COLLECTION_NAME = "test5"
NUM_REQUESTS = 5000
CONCURRENT_READS = 30

def generate_random_string(length):
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))

def send_request(url, data):
    buffer = io.BytesIO()
    c = pycurl.Curl()
    c.setopt(c.URL, url)
    c.setopt(c.POSTFIELDS, data)
    c.setopt(c.WRITEDATA, buffer)
    c.setopt(c.FOLLOWLOCATION, True)
    c.setopt(c.HTTPHEADER, [
        'Content-Type: application/x-www-form-urlencoded',
        'User-Agent: python-pycurl/7.43.0.6',
        'Accept: */*',
        ])

    try:
        c.perform()
        response_code = c.getinfo(c.RESPONSE_CODE)
        response_body = buffer.getvalue().decode('utf-8')
    except pycurl.error as e:
        response_code, response_body = None, None
        print(f"Request failed: {e}")
    finally:
        c.close()

    return response_code, response_body

def new_collection():
    data = f"create {COLLECTION_NAME}"
    status_code, _ = send_request(DB_URLS[0], data)
    return status_code

def write_operation(url):
    key = generate_random_string(8)
    value = generate_random_string(16)
    data = f"write {COLLECTION_NAME} {key} {value}"
    start_time = time.time()
    status_code, _ = send_request(url, data)
    end_time = time.time()
    duration = end_time - start_time
    return duration, status_code

def read_operation(url):
    key = generate_random_string(8)
    data = f"get {COLLECTION_NAME} {key}"
    start_time = time.perf_counter()
    status_code, _ = send_request(url, data)
    end_time = time.perf_counter()
    duration = end_time - start_time
    return duration, status_code

def write_test():
    total_time = 0.0
    success_count = 0
    for _ in range(NUM_REQUESTS):
        duration, status_code = write_operation(DB_URLS[0])
        if duration is not None and status_code is not None and status_code == 200:
            total_time += duration
            success_count += 1

    print(f"Total write time: {total_time:.2f} s")
    print(f"Total successful writes: {success_count}")

def read_test():
    total_time = 0.0
    success_count = 0
    individual_times = []

    with concurrent.futures.ThreadPoolExecutor(max_workers=CONCURRENT_READS) as executor:
        start_time = time.perf_counter()
        futures = [executor.submit(read_operation, DB_URLS[0]) for _ in range(NUM_REQUESTS)]
        future_start_time = time.perf_counter()
        for future in concurrent.futures.as_completed(futures):
            try:
                duration, status_code = future.result()
                if duration is not None and status_code is not None and status_code == 200:
                    total_time += duration
                    individual_times.append(duration)
                    success_count += 1
            except Exception as e:
                print(f"Exception during read_operation: {e}")

    duration = time.perf_counter() - start_time
    future_duration = time.perf_counter() - future_start_time
    print(f"Total function time: {duration:.2f} s")
    print(f"Total threading time: {future_duration:.2f} s")
    print(f"Total successful reads: {success_count}")

def benchmark():
    print("Starting benchmark...")
    print("requests:", NUM_REQUESTS);
    print("read threads:", CONCURRENT_READS)
    print("collection:", COLLECTION_NAME)
    print()

    print("Creating collection...")
    res = new_collection()
    if res != 200:
      exit()
    print("")
    
    print("Starting write test...")
    write_test()
    print("Write test completed.")
    print("")

    print("Starting read test...")
    read_test()
    print("Read test completed.")

benchmark()

