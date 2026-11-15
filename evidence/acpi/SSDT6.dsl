/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of SSDT6.dat
 *
 * Original Table Header:
 *     Signature        "SSDT"
 *     Length           0x000004C3 (1219)
 *     Revision         0x02
 *     Checksum         0x9D
 *     OEM ID           "Intel"
 *     OEM Table ID     "PerfTune"
 *     OEM Revision     0x00001000 (4096)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20191018 (538513432)
 */
DefinitionBlock ("", "SSDT", 2, "Intel", "PerfTune", 0x00001000)
{
    /*
     * iASL Warning: There were 3 external control methods found during
     * disassembly, but only 1 was resolved (2 unresolved). Additional
     * ACPI tables may be required to properly disassemble the code. This
     * resulting disassembler output file may not compile because the
     * disassembler did not know how many arguments to assign to the
     * unresolved methods. Note: SSDTs can be dynamically loaded at
     * runtime and may or may not be available via the host OS.
     *
     * In addition, the -fe option can be used to specify a file containing
     * control method external declarations with the associated method
     * argument counts. Each line of the file must be of the form:
     *     External (<method pathname>, MethodObj, <argument count>)
     * Invocation:
     *     iasl -fe refs.txt -d dsdt.aml
     *
     * The following methods were unresolved and many not compile properly
     * because the disassembler had to guess at the number of arguments
     * required for each:
     */
    External (_SB_.DDRF, UnknownObj)
    External (_SB_.PFTU, UnknownObj)
    External (_SB_.XMPB, UnknownObj)
    External (_SB_.XSMI, UnknownObj)
    External (_SB_.XTUB, UnknownObj)
    External (_SB_.XTUS, UnknownObj)
    External (_TZ_.TZ01._TMP, MethodObj)    // 0 Arguments
    External (ECON, IntObj)
    External (PFTU, IntObj)
    External (TSOD, IntObj)
    External (XMPB, MethodObj)    // Warning: Unknown method, guessing 1 arguments
    External (XSMI, IntObj)
    External (XTUB, MethodObj)    // Warning: Unknown method, guessing 1 arguments

    Scope (\_SB)
    {
        Device (PTMD)
        {
            Name (_HID, EisaId ("INT3394") /* ACPI System Fan */)  // _HID: Hardware ID
            Name (_CID, EisaId ("PNP0C02") /* PNP Motherboard Resources */)  // _CID: Compatible ID
            Name (IVER, 0x00010000)
            Name (SIZE, 0x055C)
            Method (GACI, 0, NotSerialized)
            {
                Name (RPKG, Package (0x02){})
                RPKG [Zero] = Zero
                If ((XTUB != Zero))
                {
                    OperationRegion (XNVS, SystemMemory, XTUB, SIZE)
                    Field (XNVS, ByteAcc, NoLock, Preserve)
                    {
                        XBUF,   13440
                    }

                    Name (TEMP, Buffer (XTUS){})
                    TEMP = XBUF /* \_SB_.PTMD.GACI.XBUF */
                    RPKG [One] = TEMP /* \_SB_.PTMD.GACI.TEMP */
                }
                Else
                {
                    RPKG [One] = Zero
                }

                Return (RPKG) /* \_SB_.PTMD.GACI.RPKG */
            }

            Method (GDSV, 1, Serialized)
            {
                If ((Arg0 == 0x05))
                {
                    Return (Package (0x02)
                    {
                        Zero, 
                        Buffer (0x58)
                        {
                            /* 0000 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                            /* 0008 */  0x01, 0x00, 0x00, 0x00, 0xB0, 0x04, 0x00, 0x00,  // ........
                            /* 0010 */  0x02, 0x00, 0x00, 0x00, 0xE2, 0x04, 0x00, 0x00,  // ........
                            /* 0018 */  0x03, 0x00, 0x00, 0x00, 0x14, 0x05, 0x00, 0x00,  // ........
                            /* 0020 */  0x04, 0x00, 0x00, 0x00, 0x46, 0x05, 0x00, 0x00,  // ....F...
                            /* 0028 */  0x05, 0x00, 0x00, 0x00, 0x78, 0x05, 0x00, 0x00,  // ....x...
                            /* 0030 */  0x06, 0x00, 0x00, 0x00, 0xAA, 0x05, 0x00, 0x00,  // ........
                            /* 0038 */  0x07, 0x00, 0x00, 0x00, 0xDC, 0x05, 0x00, 0x00,  // ........
                            /* 0040 */  0x08, 0x00, 0x00, 0x00, 0x0E, 0x06, 0x00, 0x00,  // ........
                            /* 0048 */  0x09, 0x00, 0x00, 0x00, 0x40, 0x06, 0x00, 0x00,  // ....@...
                            /* 0050 */  0x0A, 0x00, 0x00, 0x00, 0x72, 0x06, 0x00, 0x00   // ....r...
                        }
                    })
                }

                If ((Arg0 == 0x13))
                {
                    If ((DDRF == One))
                    {
                        Return (Package (0x02)
                        {
                            Zero, 
                            Buffer (0x50)
                            {
                                /* 0000 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                                /* 0008 */  0x04, 0x00, 0x00, 0x00, 0x2B, 0x04, 0x00, 0x00,  // ....+...
                                /* 0010 */  0x05, 0x00, 0x00, 0x00, 0x35, 0x05, 0x00, 0x00,  // ....5...
                                /* 0018 */  0x06, 0x00, 0x00, 0x00, 0x40, 0x06, 0x00, 0x00,  // ....@...
                                /* 0020 */  0x07, 0x00, 0x00, 0x00, 0x4B, 0x07, 0x00, 0x00,  // ....K...
                                /* 0028 */  0x08, 0x00, 0x00, 0x00, 0x55, 0x08, 0x00, 0x00,  // ....U...
                                /* 0030 */  0x09, 0x00, 0x00, 0x00, 0x60, 0x09, 0x00, 0x00,  // ....`...
                                /* 0038 */  0x0A, 0x00, 0x00, 0x00, 0x6B, 0x0A, 0x00, 0x00,  // ....k...
                                /* 0040 */  0x0B, 0x00, 0x00, 0x00, 0x75, 0x0B, 0x00, 0x00,  // ....u...
                                /* 0048 */  0x0C, 0x00, 0x00, 0x00, 0x80, 0x0C, 0x00, 0x00   // ........
                            }
                        })
                    }
                    Else
                    {
                        Return (Package (0x02)
                        {
                            Zero, 
                            Buffer (0x68)
                            {
                                /* 0000 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                                /* 0008 */  0x05, 0x00, 0x00, 0x00, 0x2B, 0x04, 0x00, 0x00,  // ....+...
                                /* 0010 */  0x06, 0x00, 0x00, 0x00, 0xB0, 0x04, 0x00, 0x00,  // ........
                                /* 0018 */  0x07, 0x00, 0x00, 0x00, 0x78, 0x05, 0x00, 0x00,  // ....x...
                                /* 0020 */  0x08, 0x00, 0x00, 0x00, 0x40, 0x06, 0x00, 0x00,  // ....@...
                                /* 0028 */  0x09, 0x00, 0x00, 0x00, 0x08, 0x07, 0x00, 0x00,  // ........
                                /* 0030 */  0x0A, 0x00, 0x00, 0x00, 0xD0, 0x07, 0x00, 0x00,  // ........
                                /* 0038 */  0x0B, 0x00, 0x00, 0x00, 0x98, 0x08, 0x00, 0x00,  // ........
                                /* 0040 */  0x0C, 0x00, 0x00, 0x00, 0x60, 0x09, 0x00, 0x00,  // ....`...
                                /* 0048 */  0x0D, 0x00, 0x00, 0x00, 0x28, 0x0A, 0x00, 0x00,  // ....(...
                                /* 0050 */  0x0E, 0x00, 0x00, 0x00, 0xF0, 0x0A, 0x00, 0x00,  // ........
                                /* 0058 */  0x0F, 0x00, 0x00, 0x00, 0xB8, 0x0B, 0x00, 0x00,  // ........
                                /* 0060 */  0x10, 0x00, 0x00, 0x00, 0x80, 0x0C, 0x00, 0x00   // ........
                            }
                        })
                    }
                }

                If ((Arg0 == 0x0B))
                {
                    Return (Package (0x02)
                    {
                        Zero, 
                        Buffer (0x60)
                        {
                            /* 0000 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                            /* 0008 */  0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,  // ........
                            /* 0010 */  0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00,  // ........
                            /* 0018 */  0x07, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,  // ........
                            /* 0020 */  0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,  // ........
                            /* 0028 */  0x0A, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00,  // ........
                            /* 0030 */  0x0C, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,  // ........
                            /* 0038 */  0x0E, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,  // ........
                            /* 0040 */  0x10, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,  // ........
                            /* 0048 */  0x12, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00,  // ........
                            /* 0050 */  0x14, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00,  // ........
                            /* 0058 */  0x18, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00   // ........
                        }
                    })
                }

                If ((Arg0 == 0x49))
                {
                    Return (Package (0x02)
                    {
                        Zero, 
                        Buffer (0x18)
                        {
                            /* 0000 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                            /* 0008 */  0x01, 0x00, 0x00, 0x00, 0x85, 0x00, 0x00, 0x00,  // ........
                            /* 0010 */  0x02, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00   // ....d...
                        }
                    })
                }

                Return (Package (0x01)
                {
                    One
                })
            }

            Method (GXDV, 1, Serialized)
            {
                If ((XMPB != Zero))
                {
                    OperationRegion (XMPN, SystemMemory, XMPB, SIZE)
                    Field (XMPN, ByteAcc, NoLock, Preserve)
                    {
                        XMP1,   832, 
                        XMP2,   832
                    }

                    If ((Arg0 == One))
                    {
                        Name (XP_1, Package (0x02){})
                        XP_1 [Zero] = Zero
                        XP_1 [One] = XMP1 /* \_SB_.PTMD.GXDV.XMP1 */
                        Return (XP_1) /* \_SB_.PTMD.GXDV.XP_1 */
                    }

                    If ((Arg0 == 0x02))
                    {
                        Name (XP_2, Package (0x02){})
                        XP_2 [Zero] = Zero
                        XP_2 [One] = XMP2 /* \_SB_.PTMD.GXDV.XMP2 */
                        Return (XP_2) /* \_SB_.PTMD.GXDV.XP_2 */
                    }
                }

                Return (Package (0x01)
                {
                    One
                })
            }

            Method (GSCV, 0, NotSerialized)
            {
                Return (PFTU) /* External reference */
            }

            Method (GSCB, 0, NotSerialized)
            {
                Return (XSMI) /* External reference */
            }

            Method (CDRD, 1, Serialized)
            {
                Return (Package (0x01)
                {
                    One
                })
            }

            Method (CDWR, 2, Serialized)
            {
                Return (One)
            }

            Name (RPMV, Package (0x04)
            {
                One, 
                0x07, 
                Zero, 
                Zero
            })
            Name (TMP1, Package (0x0C)
            {
                One, 
                0x02, 
                Zero, 
                Zero, 
                0x05, 
                0x04, 
                Zero, 
                Zero, 
                0x06, 
                0x05, 
                Zero, 
                Zero
            })
            Name (TMP2, Package (0x08)
            {
                One, 
                0x02, 
                Zero, 
                Zero, 
                0x05, 
                0x04, 
                Zero, 
                Zero
            })
            Name (TMP3, Package (0x04)
            {
                One, 
                0x02, 
                Zero, 
                Zero
            })
            Method (TSDD, 0, NotSerialized)
            {
                If ((XTUS == Zero))
                {
                    Return (Zero)
                }

                If (\ECON)
                {
                    If (\TSOD)
                    {
                        TMP1 [0x02] = \_TZ.TZ01._TMP ()
                        Return (TMP1) /* \_SB_.PTMD.TMP1 */
                    }
                    Else
                    {
                        TMP2 [0x02] = \_TZ.TZ01._TMP ()
                        Return (TMP2) /* \_SB_.PTMD.TMP2 */
                    }
                }
                Else
                {
                    TMP3 [0x02] = \_TZ.TZ01._TMP ()
                    Return (TMP3) /* \_SB_.PTMD.TMP3 */
                }
            }

            Method (FSDD, 0, NotSerialized)
            {
                If ((XTUS == Zero))
                {
                    Return (Zero)
                }
            }

            Method (SDSP, 0, NotSerialized)
            {
                Return (0x0A)
            }
        }
    }
}

