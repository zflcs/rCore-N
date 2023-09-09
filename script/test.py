import socket
import time


def main():
    tcp_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    time.sleep(0.2 * 2)
    server_addr = ("192.168.1.2", 80)
    tcp_socket.connect(server_addr)
    send_data = "connect ok?"
    tcp_socket.send(send_data.encode("utf8"))
    recv_data = tcp_socket.recv(1024)
    print('recv connect result:', recv_data.decode("utf8"))

main()