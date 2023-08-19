import shutil
import copy
import json
import os

if __name__ == "__main__":
    # create json file from original model
    os.system("./flatc -t --strict-json --raw-binary --defaults-json schema.fbs -- model_original.tflite")

    # model_original : unmodified version
    model_original = None
    with open("model_original.json", "r") as file: # create and open file
        model_original = json.load(file)

    # model_local : client/server side version
    model_local = copy.deepcopy(model_original)
    with open("model_local.json", "w") as file: # edit the local model
        # change the overall outputs
        model_local["subgraphs"][0]["outputs"] = [333]

        # create new buffer
        buffers = model_local["buffers"]
        buffers += [{}]

        model_local["buffers"] = buffers # list of buffers

        # create new tensor
        tensors = model_local["subgraphs"][0]["tensors"]
        tensors += [{
            'shape': [1, 96, 96, 16],
            'type': 'FLOAT32',
            'buffer': 335,
            'name': 'StatefulPartitionedCall:0',
            'quantization': {
                'details_type': 'NONE',
                'quantized_dimension': 0
            },
            'is_variable': False,
            'has_rank': False
        }]

        model_local["subgraphs"][0]["tensors"] = tensors # list of tensors

        # change the operators vector
        operators = model_local["subgraphs"][0]["operators"][:8]
        operators += [{
            'opcode_index': 13,
            'inputs': [181],
            'outputs': [333],
            'builtin_options_type': 'NONE',
            'custom_options_format': 'FLEXBUFFERS'
        }]

        model_local["subgraphs"][0]["operators"] = operators # list of operators
        
        # breakpoint() # breakpoint

        json.dump(model_local, file) # put changes to new json
    
    # find instances where precision is wrong and fix
    with open("model_local.json", "r") as file:
        model_local = file.read().replace("0.0039062", "0.00390625")
    
    with open("model_local.json", "w") as file:
        file.write(model_local)

    # model_remote : remote server version
    model_remote = copy.deepcopy(model_original)
    with open("model_remote.json", "w") as file: # edit the remote model
        # change the overall inputs
        model_remote["subgraphs"][0]["inputs"] = [333]

        # create new buffer
        buffers = model_remote["buffers"]
        buffers += [{}]

        model_remote["buffers"] = buffers # list of buffers

        # create new tensor
        tensors = model_remote["subgraphs"][0]["tensors"]
        tensors += [{
            'shape': [1, 96, 96, 16],
            'type': 'FLOAT32',
            'buffer': 335,
            'name': 'StatefulPartitionedCall:0',
            'quantization': {
                'details_type': 'NONE',
                'quantized_dimension': 0
            },
            'is_variable': False,
            'has_rank': False
        }]

        model_remote["subgraphs"][0]["tensors"] = tensors # list of tensors

        # change operators vector
        operators = model_remote["subgraphs"][0]["operators"][8:]
        operators[:0] = [{
            'opcode_index': 1,
            'inputs': [333],
            'outputs': [181],
            'builtin_options_type': 'NONE',
            'custom_options_format': 'FLEXBUFFERS'
        }]

        model_remote["subgraphs"][0]["operators"] = operators # list of operators

        # breakpoint() # breakpoint

        json.dump(model_remote, file) # put changes to new json

    # find instances where precision is wrong and fix
    with open("model_remote.json", "r") as file:
        model_remote = file.read().replace("0.0039062", "0.00390625")

    with open("model_remote.json", "w") as file:
        file.write(model_remote)

    # generate the resulting flatc files
    os.system("./flatc -b --strict-json --defaults-json -o flatc_local schema.fbs model_local.json")
    os.system("./flatc -b --strict-json --defaults-json -o flatc_remote schema.fbs model_remote.json")

    # replace the files used by the two components
    shutil.copy("flatc_local/model_local.tflite", "../client_side/resource")
    shutil.copy("flatc_remote/model_remote.tflite", "../remote_server/resource")
