# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: connect.proto
# Protobuf Python Version: 4.25.0
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
from google.protobuf.internal import builder as _builder
# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()




DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\rconnect.proto\x12\x0e\x66lexim_connect\" \n\x10\x43reateBagRequest\x12\x0c\n\x04name\x18\x01 \x01(\t\"-\n\x11\x43reateBagResponse\x12\n\n\x02id\x18\x01 \x01(\x04\x12\x0c\n\x04name\x18\x02 \x01(\t\"\xde\x03\n\x11\x41ppendDataRequest\x12:\n\x04meta\x18\x01 \x01(\x0b\x32*.flexim_connect.AppendDataRequest.DataMetaH\x00\x12\x14\n\ndata_bytes\x18\x02 \x01(\x0cH\x00\x1a\xee\x02\n\x08\x44\x61taMeta\x12\x0e\n\x06\x62\x61g_id\x18\x01 \x01(\x04\x12\x0c\n\x04name\x18\x02 \x01(\t\x12+\n\tdata_type\x18\x04 \x01(\x0e\x32\x18.flexim_connect.DataType\x12W\n\x0fspecial_columns\x18\x05 \x03(\x0b\x32>.flexim_connect.AppendDataRequest.DataMeta.SpecialColumnsEntry\x1ao\n\x13SpecialColumnsEntry\x12\x0b\n\x03key\x18\x01 \x01(\t\x12G\n\x05value\x18\x02 \x01(\x0e\x32\x38.flexim_connect.AppendDataRequest.DataMeta.SpecialColumn:\x02\x38\x01\"M\n\rSpecialColumn\x12\r\n\tRectangle\x10\x00\x12\x0b\n\x07Segment\x10\x01\x12\t\n\x05\x43olor\x10\x02\x12\n\n\x06Vector\x10\x03\x12\t\n\x05Point\x10\x04\x42\x06\n\x04\x64\x61ta\"H\n\x12\x41ppendDataResponse\x12\x0e\n\x06\x62\x61g_id\x18\x01 \x01(\x04\x12\x0f\n\x07\x64\x61ta_id\x18\x02 \x01(\x04\x12\x11\n\tdata_size\x18\x03 \x01(\x04\"\x11\n\x0fListBagsRequest\"\xa0\x01\n\x10ListBagsResponse\x12;\n\tbag_metas\x18\x01 \x03(\x0b\x32(.flexim_connect.ListBagsResponse.BagMeta\x1aO\n\x07\x42\x61gMeta\x12\n\n\x02id\x18\x01 \x01(\x04\x12\x0c\n\x04name\x18\x02 \x01(\t\x12\x16\n\x0enumber_of_data\x18\x03 \x01(\x04\x12\x12\n\ncreated_at\x18\x04 \x01(\t*2\n\x08\x44\x61taType\x12\t\n\x05Image\x10\x00\x12\x0c\n\x08Tensor2D\x10\x01\x12\r\n\tDataFrame\x10\x02\x32\x8d\x02\n\rFleximConnect\x12R\n\tCreateBag\x12 .flexim_connect.CreateBagRequest\x1a!.flexim_connect.CreateBagResponse\"\x00\x12W\n\nAppendData\x12!.flexim_connect.AppendDataRequest\x1a\".flexim_connect.AppendDataResponse\"\x00(\x01\x12O\n\x08ListBags\x12\x1f.flexim_connect.ListBagsRequest\x1a .flexim_connect.ListBagsResponse\"\x00\x62\x06proto3')

_globals = globals()
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, _globals)
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'connect_pb2', _globals)
if _descriptor._USE_C_DESCRIPTORS == False:
  DESCRIPTOR._options = None
  _globals['_APPENDDATAREQUEST_DATAMETA_SPECIALCOLUMNSENTRY']._options = None
  _globals['_APPENDDATAREQUEST_DATAMETA_SPECIALCOLUMNSENTRY']._serialized_options = b'8\001'
  _globals['_DATATYPE']._serialized_start=851
  _globals['_DATATYPE']._serialized_end=901
  _globals['_CREATEBAGREQUEST']._serialized_start=33
  _globals['_CREATEBAGREQUEST']._serialized_end=65
  _globals['_CREATEBAGRESPONSE']._serialized_start=67
  _globals['_CREATEBAGRESPONSE']._serialized_end=112
  _globals['_APPENDDATAREQUEST']._serialized_start=115
  _globals['_APPENDDATAREQUEST']._serialized_end=593
  _globals['_APPENDDATAREQUEST_DATAMETA']._serialized_start=219
  _globals['_APPENDDATAREQUEST_DATAMETA']._serialized_end=585
  _globals['_APPENDDATAREQUEST_DATAMETA_SPECIALCOLUMNSENTRY']._serialized_start=395
  _globals['_APPENDDATAREQUEST_DATAMETA_SPECIALCOLUMNSENTRY']._serialized_end=506
  _globals['_APPENDDATAREQUEST_DATAMETA_SPECIALCOLUMN']._serialized_start=508
  _globals['_APPENDDATAREQUEST_DATAMETA_SPECIALCOLUMN']._serialized_end=585
  _globals['_APPENDDATARESPONSE']._serialized_start=595
  _globals['_APPENDDATARESPONSE']._serialized_end=667
  _globals['_LISTBAGSREQUEST']._serialized_start=669
  _globals['_LISTBAGSREQUEST']._serialized_end=686
  _globals['_LISTBAGSRESPONSE']._serialized_start=689
  _globals['_LISTBAGSRESPONSE']._serialized_end=849
  _globals['_LISTBAGSRESPONSE_BAGMETA']._serialized_start=770
  _globals['_LISTBAGSRESPONSE_BAGMETA']._serialized_end=849
  _globals['_FLEXIMCONNECT']._serialized_start=904
  _globals['_FLEXIMCONNECT']._serialized_end=1173
# @@protoc_insertion_point(module_scope)
