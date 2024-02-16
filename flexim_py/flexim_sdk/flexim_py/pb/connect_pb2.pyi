from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import (
    ClassVar as _ClassVar,
    Iterable as _Iterable,
    Mapping as _Mapping,
    Optional as _Optional,
    Union as _Union,
)

DESCRIPTOR: _descriptor.FileDescriptor

class DataType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    Image: _ClassVar[DataType]
    Tensor2D: _ClassVar[DataType]
    DataFrame: _ClassVar[DataType]

Image: DataType
Tensor2D: DataType
DataFrame: DataType

class CreateBagRequest(_message.Message):
    __slots__ = ("name",)
    NAME_FIELD_NUMBER: _ClassVar[int]
    name: str
    def __init__(self, name: _Optional[str] = ...) -> None: ...

class CreateBagResponse(_message.Message):
    __slots__ = ("id", "name")
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    id: int
    name: str
    def __init__(self, id: _Optional[int] = ..., name: _Optional[str] = ...) -> None: ...

class AppendDataRequest(_message.Message):
    __slots__ = ("meta", "data_bytes")
    class DataMeta(_message.Message):
        __slots__ = ("bag_id", "name", "data_type", "special_columns")
        class SpecialColumn(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
            __slots__ = ()
            Rectangle: _ClassVar[AppendDataRequest.DataMeta.SpecialColumn]
            Segment: _ClassVar[AppendDataRequest.DataMeta.SpecialColumn]
            Color: _ClassVar[AppendDataRequest.DataMeta.SpecialColumn]
            Vector: _ClassVar[AppendDataRequest.DataMeta.SpecialColumn]
            Point: _ClassVar[AppendDataRequest.DataMeta.SpecialColumn]
        Rectangle: AppendDataRequest.DataMeta.SpecialColumn
        Segment: AppendDataRequest.DataMeta.SpecialColumn
        Color: AppendDataRequest.DataMeta.SpecialColumn
        Vector: AppendDataRequest.DataMeta.SpecialColumn
        Point: AppendDataRequest.DataMeta.SpecialColumn
        class SpecialColumnsEntry(_message.Message):
            __slots__ = ("key", "value")
            KEY_FIELD_NUMBER: _ClassVar[int]
            VALUE_FIELD_NUMBER: _ClassVar[int]
            key: str
            value: AppendDataRequest.DataMeta.SpecialColumn
            def __init__(
                self,
                key: _Optional[str] = ...,
                value: _Optional[_Union[AppendDataRequest.DataMeta.SpecialColumn, str]] = ...,
            ) -> None: ...
        BAG_ID_FIELD_NUMBER: _ClassVar[int]
        NAME_FIELD_NUMBER: _ClassVar[int]
        DATA_TYPE_FIELD_NUMBER: _ClassVar[int]
        SPECIAL_COLUMNS_FIELD_NUMBER: _ClassVar[int]
        bag_id: int
        name: str
        data_type: DataType
        special_columns: _containers.ScalarMap[str, AppendDataRequest.DataMeta.SpecialColumn]
        def __init__(
            self,
            bag_id: _Optional[int] = ...,
            name: _Optional[str] = ...,
            data_type: _Optional[_Union[DataType, str]] = ...,
            special_columns: _Optional[_Mapping[str, AppendDataRequest.DataMeta.SpecialColumn]] = ...,
        ) -> None: ...
    META_FIELD_NUMBER: _ClassVar[int]
    DATA_BYTES_FIELD_NUMBER: _ClassVar[int]
    meta: AppendDataRequest.DataMeta
    data_bytes: bytes
    def __init__(
        self,
        meta: _Optional[_Union[AppendDataRequest.DataMeta, _Mapping]] = ...,
        data_bytes: _Optional[bytes] = ...,
    ) -> None: ...

class AppendDataResponse(_message.Message):
    __slots__ = ("bag_id", "data_id", "data_size")
    BAG_ID_FIELD_NUMBER: _ClassVar[int]
    DATA_ID_FIELD_NUMBER: _ClassVar[int]
    DATA_SIZE_FIELD_NUMBER: _ClassVar[int]
    bag_id: int
    data_id: int
    data_size: int
    def __init__(
        self,
        bag_id: _Optional[int] = ...,
        data_id: _Optional[int] = ...,
        data_size: _Optional[int] = ...,
    ) -> None: ...

class ListBagsRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ListBagsResponse(_message.Message):
    __slots__ = ("bag_metas",)
    class BagMeta(_message.Message):
        __slots__ = ("id", "name", "number_of_data", "created_at")
        ID_FIELD_NUMBER: _ClassVar[int]
        NAME_FIELD_NUMBER: _ClassVar[int]
        NUMBER_OF_DATA_FIELD_NUMBER: _ClassVar[int]
        CREATED_AT_FIELD_NUMBER: _ClassVar[int]
        id: int
        name: str
        number_of_data: int
        created_at: str
        def __init__(
            self,
            id: _Optional[int] = ...,
            name: _Optional[str] = ...,
            number_of_data: _Optional[int] = ...,
            created_at: _Optional[str] = ...,
        ) -> None: ...
    BAG_METAS_FIELD_NUMBER: _ClassVar[int]
    bag_metas: _containers.RepeatedCompositeFieldContainer[ListBagsResponse.BagMeta]
    def __init__(
        self,
        bag_metas: _Optional[_Iterable[_Union[ListBagsResponse.BagMeta, _Mapping]]] = ...,
    ) -> None: ...
