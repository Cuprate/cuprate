# MLSAG Rules

## Introduction

These rules are split into 3 sections: Full, Simple and Both. Full is for RCT type Full and Simple are for the other RCT types
that use MLSAG signatures.

> Simple is not just for RCT type Simple!

## Index

1. [Full Rules](#full-rules)
2. [Simple Rules](#simple-rules)

## Full Rules

### Creating The Ring Matrix (Full)

For RCT type full the ring matrix contains every inputs ring members: [^full-matrix]

(The signer owns a whole column)

```bob
         .-------.-------.-------.- - - -.     
         | I1 R1 | I1 R2 | I1 R3 | ..... |  
         | I2 R1 | I2 R2 | I2 R3 | ..... |    
         | I3 R1 | I3 R2 | I3 R3 | ..... |
           .....   .....   .....   .....  
         |   A   |   A   |   A   | ..... | <-.
         '-------'-------'-------'-------'   |              
                                             |  
I = Input                                    |
R = Ring member                              |
A = Pedersen Commitment                      |
```

The last row contains: \\(\sum CommitmentsAtIndex - \sum outPK - fee * H \\) [^full-last-row]

Where CommitmentsAtIndex are the ring members commitments in that column.

Which means that for the true spends column the entry in the last row will be commitment to 0.

By structuring the matrix like this the true spend has to be a the same index in each inputs ring,
which is not good for privacy.

### Number Of Ring Members

There must be the same amount of ring members in each inputs ring.[^full-numb-ring-members]

### One MLSAGs

There must be only one MLSAG signature.[^numb-mlsags]

## Simple Rules

### Creating The Ring Matrix (Simple)

For simple RCT types the ring matrix only contains the ring members of a single input: [^simple-matrix]

```bob
         .-------.-------.-------.- - - -.     
         | IX R1 | IX R2 | IX R3 | ..... |     
         |   A   |   A   |   A   | ..... | <-.
         '-------'-------'-------'- - - -'   |             
                                             |  
I = Input                                    |
R = Ring member                              |
A = Pedersen Commitment                      |
```

The last row contains the ring members commitment minus the pseudo-out for this input.[^simple-last-row]

### Simple Number Of MLSAGs

There must be the same amount of MLSAG signatures as there are inputs.[^numb-mlsags]

## Rules That Apply To Both

### More Than One Ring Member

There must be more than one ring member.[^more-than-one-ring-member]

### SS Size

The ss field must be the same length as the key matrix[^ss-size] and each ss member lengths must be the same as the matrix's rows. [^ss-member-size]

### SS, CC Canonical Encoding

Every ss element and cc must be fully reduced scalars.[^ss-cc-reduced]

### Key Images Not Identity

All the key images must not be equal to the identity point.[^ki-not-identity]

### The MLSAG Signature Must Be Correct

The signature must be valid.[^mlsag-valid]

---

[^full-matrix]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L802>

[^full-last-row]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L827-L833>

[^full-numb-ring-members]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L810>

[^numb-mlsags]: <https://github.com/monero-project/monero/blame/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctTypes.h#L537-L540C28>s

[^simple-matrix]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L841>

[^simple-last-row]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L861-L864>

[^more-than-one-ring-member]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L462>

[^ss-size]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L469>

[^ss-member-size]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L471>

[^ss-cc-reduced]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L477-L480>

[^ki-not-identity]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L487>

[^mlsag-valid]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L460>
