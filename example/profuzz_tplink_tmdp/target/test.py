import struct


class Paket:
    def _init_from_raw(self, raw: bytearray):
        self._cursor: int = 0
        self._raw: bytearray = raw

    def _consume(self, comsume_len: int = 1, interpret=None):
        self._cursor += comsume_len
        ret = self._raw[self._cursor - comsume_len : self._cursor]
        print(ret)
        if len(ret) == 0:
            return None
        if type(interpret) == str:
            return struct.unpack(interpret, ret)[0]
        if interpret:
            return interpret(ret)
        return ret

    def _remaining_raw_data(self) -> bytearray:
        return self._raw[self._cursor :]


class TetherHelloFromClient(Paket):
    def __init__(self, raw: bytearray):
        self._init_from_raw(raw)


class TetherHelloFromServer(Paket):
    def __init__(self, raw: bytearray):
        self._init_from_raw(raw)


class TetherUnknown:
    pass


class TetherData(Paket):
    pkt_max_len = 3072  # getTmpPktMaxSize
    pkt_header_size = 16  # getTmpPktHdrSize

    def __init__(self, raw: bytearray):
        self._init_from_raw(raw)

        _ = self._consume(4)  # HEADER

        l = self._consume(2)  # pkt[4:6]
        b = self._consume(2)  # pkt[6:8]

        self.size = struct.unpack(">I", b + l)[0]

        self.data_type = self._consume(4, ">I")  # pkt[8:12]
        print("self.data_type", self.data_type)
        self.header_checksum = self._consume(4, ">I")  # pkt[12:16]

        self.options = self._consume(2, bytes)
        # should \x01 \x01
        self.jump_table = self._consume(2, ">H")
        print("self.function_pointer", hex(self.jump_table))

        # self.header_size = self._consume(4) # pkt[16:19]
        # self.header_flag = self._consume(4, "<I") # pkt[19:23]

        # if self.header_flag == 1:
        #     self.header_type = self.header_type << 0x18 | self.header_type >> 0x18 | (self.header_type >> 8) & 0xff00 | (self.header_type & 0xff00) << 8

        self.data = self._consume(self.size - 8)

    def is_checksum_valid(self):
        print(self.size + 16)
        print(self._raw[0 : self.size + 16])
        return tmp_crc32(self._raw[0 : self.size + 16]) == self.header_checksum

    def update_checksum(self):
        self.header_checksum = tmp_crc32(self._raw)


class TetherPaket(Paket):
    pkt_header_size = 4  # getTmpGpHdrSize

    def __init__(self, raw: bytearray):
        self.version = 0
        self.type = TetherUnknown
        self.payload = TetherUnknown()

        self.parse_from_raw(raw)

    def parse_from_raw(self, raw: bytearray):
        self._init_from_raw(raw)

        if type(raw) == bytearray and len(raw) >= self.pkt_header_size:
            self.version = self._consume(2, bytes)
            self.type = self._get_type(self._consume(1, ord))
            self._consume(1)  # RESERVED

        if self.type != TetherUnknown:
            self.payload = self.type(self._raw)

    def _remaining_raw_data(self) -> bytearray:
        if self.type == TetherData:
            return self.payload._remaining_raw_data()
        return super()._remaining_raw_data()

    def _get_type(self, pkt_type: bytearray):
        if pkt_type == 0:
            # return 0
            pass

        if pkt_type == 1:
            return TetherHelloFromClient

        if pkt_type == 2:
            return TetherHelloFromServer

        if pkt_type == 3:
            pass

        if pkt_type == 4:
            pass

        if pkt_type == 5:
            return TetherData

        if pkt_type == 6:
            pass

        return TetherUnknown


import struct
import numpy as np


class hexdump:
    def __init__(self, buf, off=0):
        self.buf = buf
        self.off = off

    def __iter__(self):
        last_bs, last_line = None, None
        for i in range(0, len(self.buf), 16):
            bs = bytearray(self.buf[i : i + 16])
            line = "{:08x}  {:23}  {:23}  |{:16}|".format(
                self.off + i,
                " ".join(("{:02x}".format(x) for x in bs[:8])),
                " ".join(("{:02x}".format(x) for x in bs[8:])),
                "".join((chr(x) if 32 <= x < 127 else "." for x in bs)),
            )
            if bs == last_bs:
                line = "*"
            if bs != last_bs or line != last_line:
                yield line
            last_bs, last_line = bs, line
        yield "{:08x}".format(self.off + len(self.buf))

    def __str__(self):
        return "\n".join(self)

    def __repr__(self):
        return "\n".join(self)


CRC32_TABLE = [
    0x00000000,
    0x96300777,
    0x2C610EEE,
    0xBA510999,
    0x19C46D07,
    0x8FF46A70,
    0x35A563E9,
    0xA395649E,
    0x3288DB0E,
    0xA4B8DC79,
    0x1EE9D5E0,
    0x88D9D297,
    0x2B4CB609,
    0xBD7CB17E,
    0x072DB8E7,
    0x911DBF90,
    0x6410B71D,
    0xF220B06A,
    0x4871B9F3,
    0xDE41BE84,
    0x7DD4DA1A,
    0xEBE4DD6D,
    0x51B5D4F4,
    0xC785D383,
    0x56986C13,
    0xC0A86B64,
    0x7AF962FD,
    0xECC9658A,
    0x4F5C0114,
    0xD96C0663,
    0x633D0FFA,
    0xF50D088D,
    0xC8206E3B,
    0x5E10694C,
    0xE44160D5,
    0x727167A2,
    0xD1E4033C,
    0x47D4044B,
    0xFD850DD2,
    0x6BB50AA5,
    0xFAA8B535,
    0x6C98B242,
    0xD6C9BBDB,
    0x40F9BCAC,
    0xE36CD832,
    0x755CDF45,
    0xCF0DD6DC,
    0x593DD1AB,
    0xAC30D926,
    0x3A00DE51,
    0x8051D7C8,
    0x1661D0BF,
    0xB5F4B421,
    0x23C4B356,
    0x9995BACF,
    0x0FA5BDB8,
    0x9EB80228,
    0x0888055F,
    0xB2D90CC6,
    0x24E90BB1,
    0x877C6F2F,
    0x114C6858,
    0xAB1D61C1,
    0x3D2D66B6,
    0x9041DC76,
    0x0671DB01,
    0xBC20D298,
    0x2A10D5EF,
    0x8985B171,
    0x1FB5B606,
    0xA5E4BF9F,
    0x33D4B8E8,
    0xA2C90778,
    0x34F9000F,
    0x8EA80996,
    0x18980EE1,
    0xBB0D6A7F,
    0x2D3D6D08,
    0x976C6491,
    0x015C63E6,
    0xF4516B6B,
    0x62616C1C,
    0xD8306585,
    0x4E0062F2,
    0xED95066C,
    0x7BA5011B,
    0xC1F40882,
    0x57C40FF5,
    0xC6D9B065,
    0x50E9B712,
    0xEAB8BE8B,
    0x7C88B9FC,
    0xDF1DDD62,
    0x492DDA15,
    0xF37CD38C,
    0x654CD4FB,
    0x5861B24D,
    0xCE51B53A,
    0x7400BCA3,
    0xE230BBD4,
    0x41A5DF4A,
    0xD795D83D,
    0x6DC4D1A4,
    0xFBF4D6D3,
    0x6AE96943,
    0xFCD96E34,
    0x468867AD,
    0xD0B860DA,
    0x732D0444,
    0xE51D0333,
    0x5F4C0AAA,
    0xC97C0DDD,
    0x3C710550,
    0xAA410227,
    0x10100BBE,
    0x86200CC9,
    0x25B56857,
    0xB3856F20,
    0x09D466B9,
    0x9FE461CE,
    0x0EF9DE5E,
    0x98C9D929,
    0x2298D0B0,
    0xB4A8D7C7,
    0x173DB359,
    0x810DB42E,
    0x3B5CBDB7,
    0xAD6CBAC0,
    0x2083B8ED,
    0xB6B3BF9A,
    0x0CE2B603,
    0x9AD2B174,
    0x3947D5EA,
    0xAF77D29D,
    0x1526DB04,
    0x8316DC73,
    0x120B63E3,
    0x843B6494,
    0x3E6A6D0D,
    0xA85A6A7A,
    0x0BCF0EE4,
    0x9DFF0993,
    0x27AE000A,
    0xB19E077D,
    0x44930FF0,
    0xD2A30887,
    0x68F2011E,
    0xFEC20669,
    0x5D5762F7,
    0xCB676580,
    0x71366C19,
    0xE7066B6E,
    0x761BD4FE,
    0xE02BD389,
    0x5A7ADA10,
    0xCC4ADD67,
    0x6FDFB9F9,
    0xF9EFBE8E,
    0x43BEB717,
    0xD58EB060,
    0xE8A3D6D6,
    0x7E93D1A1,
    0xC4C2D838,
    0x52F2DF4F,
    0xF167BBD1,
    0x6757BCA6,
    0xDD06B53F,
    0x4B36B248,
    0xDA2B0DD8,
    0x4C1B0AAF,
    0xF64A0336,
    0x607A0441,
    0xC3EF60DF,
    0x55DF67A8,
    0xEF8E6E31,
    0x79BE6946,
    0x8CB361CB,
    0x1A8366BC,
    0xA0D26F25,
    0x36E26852,
    0x95770CCC,
    0x03470BBB,
    0xB9160222,
    0x2F260555,
    0xBE3BBAC5,
    0x280BBDB2,
    0x925AB42B,
    0x046AB35C,
    0xA7FFD7C2,
    0x31CFD0B5,
    0x8B9ED92C,
    0x1DAEDE5B,
    0xB0C2649B,
    0x26F263EC,
    0x9CA36A75,
    0x0A936D02,
    0xA906099C,
    0x3F360EEB,
    0x85670772,
    0x13570005,
    0x824ABF95,
    0x147AB8E2,
    0xAE2BB17B,
    0x381BB60C,
    0x9B8ED292,
    0x0DBED5E5,
    0xB7EFDC7C,
    0x21DFDB0B,
    0xD4D2D386,
    0x42E2D4F1,
    0xF8B3DD68,
    0x6E83DA1F,
    0xCD16BE81,
    0x5B26B9F6,
    0xE177B06F,
    0x7747B718,
    0xE65A0888,
    0x706A0FFF,
    0xCA3B0666,
    0x5C0B0111,
    0xFF9E658F,
    0x69AE62F8,
    0xD3FF6B61,
    0x45CF6C16,
    0x78E20AA0,
    0xEED20DD7,
    0x5483044E,
    0xC2B30339,
    0x612667A7,
    0xF71660D0,
    0x4D476949,
    0xDB776E3E,
    0x4A6AD1AE,
    0xDC5AD6D9,
    0x660BDF40,
    0xF03BD837,
    0x53AEBCA9,
    0xC59EBBDE,
    0x7FCFB247,
    0xE9FFB530,
    0x1CF2BDBD,
    0x8AC2BACA,
    0x3093B353,
    0xA6A3B424,
    0x0536D0BA,
    0x9306D7CD,
    0x2957DE54,
    0xBF67D923,
    0x2E7A66B3,
    0xB84A61C4,
    0x021B685D,
    0x942B6F2A,
    0x37BE0BB4,
    0xA18E0CC3,
    0x1BDF055A,
    0x8DEF022D,
]


def swap_endiannes(byte: int):
    return (
        (byte << 24 & 0xFFFFFFFF)
        | (byte >> 24 & 0xFFFFFFFF)
        | byte >> 8 & 0xFF00
        | (byte & 0xFF00) << 8
    )


def tmp_crc32(raw_bytes: bytearray):
    checksum = 0x8D7C6B5A
    checksum = struct.pack("<I", checksum)

    raw_bytes[12] = checksum[0]
    raw_bytes[13] = checksum[1]
    raw_bytes[14] = checksum[2]
    raw_bytes[15] = checksum[3]

    print([x for x in raw_bytes])

    current_byte = np.uint32(0xFFFFFFFF)

    for i in range(0, len(raw_bytes)):
        raw_byte = raw_bytes[i]

        a = CRC32_TABLE[(raw_byte ^ current_byte) & 0xFF]
        a = swap_endiannes(a)

        current_byte = np.uint32(a ^ (current_byte >> 8))

    return ~current_byte


b = bytes.fromhex(
    "010005000008000000000003855e81d40101080000000000010005000008000000000002f9537b370101020000000000010005000020000000000005f6e4e2ca01010300000000000310000400000000ff0000000311000400000010ff00000001000500000800000000000482edd4ec01010802000000000100050000200000000000066a8a6fe201010302000000000310000400000000ff0000000311000400000010ff000000"
)
tpkt = TetherPaket(bytearray(b))
print(tpkt.payload.is_checksum_valid())
print(tpkt.payload.header_checksum)
