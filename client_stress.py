import threading,subprocess

def run():
    p=subprocess.Popen("target/debug/client.exe")
    p.wait()

threads = []
for i in range(10):
    t = threading.Thread(target=run)
    t.start()
    threads.append(t)
    
for t in threads:
    t.join()

print("done")